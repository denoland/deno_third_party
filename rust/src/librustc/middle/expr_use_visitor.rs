// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A different sort of visitor for walking fn bodies.  Unlike the
//! normal visitor, which just walks the entire body in one shot, the
//! `ExprUseVisitor` determines how expressions are being used.

pub use self::LoanCause::*;
pub use self::ConsumeMode::*;
pub use self::MoveReason::*;
pub use self::MatchMode::*;
use self::TrackMatchMode::*;
use self::OverloadedCallType::*;

use hir::def::Def;
use hir::def_id::DefId;
use infer::InferCtxt;
use middle::mem_categorization as mc;
use middle::region;
use ty::{self, TyCtxt, adjustment};

use hir::{self, PatKind};
use rustc_data_structures::sync::Lrc;
use std::rc::Rc;
use syntax::ast;
use syntax::ptr::P;
use syntax_pos::Span;
use util::nodemap::ItemLocalSet;

///////////////////////////////////////////////////////////////////////////
// The Delegate trait

/// This trait defines the callbacks you can expect to receive when
/// employing the ExprUseVisitor.
pub trait Delegate<'tcx> {
    // The value found at `cmt` is either copied or moved, depending
    // on mode.
    fn consume(&mut self,
               consume_id: ast::NodeId,
               consume_span: Span,
               cmt: &mc::cmt_<'tcx>,
               mode: ConsumeMode);

    // The value found at `cmt` has been determined to match the
    // pattern binding `matched_pat`, and its subparts are being
    // copied or moved depending on `mode`.  Note that `matched_pat`
    // is called on all variant/structs in the pattern (i.e., the
    // interior nodes of the pattern's tree structure) while
    // consume_pat is called on the binding identifiers in the pattern
    // (which are leaves of the pattern's tree structure).
    //
    // Note that variants/structs and identifiers are disjoint; thus
    // `matched_pat` and `consume_pat` are never both called on the
    // same input pattern structure (though of `consume_pat` can be
    // called on a subpart of an input passed to `matched_pat).
    fn matched_pat(&mut self,
                   matched_pat: &hir::Pat,
                   cmt: &mc::cmt_<'tcx>,
                   mode: MatchMode);

    // The value found at `cmt` is either copied or moved via the
    // pattern binding `consume_pat`, depending on mode.
    fn consume_pat(&mut self,
                   consume_pat: &hir::Pat,
                   cmt: &mc::cmt_<'tcx>,
                   mode: ConsumeMode);

    // The value found at `borrow` is being borrowed at the point
    // `borrow_id` for the region `loan_region` with kind `bk`.
    fn borrow(&mut self,
              borrow_id: ast::NodeId,
              borrow_span: Span,
              cmt: &mc::cmt_<'tcx>,
              loan_region: ty::Region<'tcx>,
              bk: ty::BorrowKind,
              loan_cause: LoanCause);

    // The local variable `id` is declared but not initialized.
    fn decl_without_init(&mut self,
                         id: ast::NodeId,
                         span: Span);

    // The path at `cmt` is being assigned to.
    fn mutate(&mut self,
              assignment_id: ast::NodeId,
              assignment_span: Span,
              assignee_cmt: &mc::cmt_<'tcx>,
              mode: MutateMode);
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LoanCause {
    ClosureCapture(Span),
    AddrOf,
    AutoRef,
    AutoUnsafe,
    RefBinding,
    OverloadedOperator,
    ClosureInvocation,
    ForLoop,
    MatchDiscriminant
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ConsumeMode {
    Copy,                // reference to x where x has a type that copies
    Move(MoveReason),    // reference to x where x has a type that moves
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MoveReason {
    DirectRefMove,
    PatBindingMove,
    CaptureMove,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MatchMode {
    NonBindingMatch,
    BorrowingMatch,
    CopyingMatch,
    MovingMatch,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum TrackMatchMode {
    Unknown,
    Definite(MatchMode),
    Conflicting,
}

impl TrackMatchMode {
    // Builds up the whole match mode for a pattern from its constituent
    // parts.  The lattice looks like this:
    //
    //          Conflicting
    //            /     \
    //           /       \
    //      Borrowing   Moving
    //           \       /
    //            \     /
    //            Copying
    //               |
    //          NonBinding
    //               |
    //            Unknown
    //
    // examples:
    //
    // * `(_, some_int)` pattern is Copying, since
    //   NonBinding + Copying => Copying
    //
    // * `(some_int, some_box)` pattern is Moving, since
    //   Copying + Moving => Moving
    //
    // * `(ref x, some_box)` pattern is Conflicting, since
    //   Borrowing + Moving => Conflicting
    //
    // Note that the `Unknown` and `Conflicting` states are
    // represented separately from the other more interesting
    // `Definite` states, which simplifies logic here somewhat.
    fn lub(&mut self, mode: MatchMode) {
        *self = match (*self, mode) {
            // Note that clause order below is very significant.
            (Unknown, new) => Definite(new),
            (Definite(old), new) if old == new => Definite(old),

            (Definite(old), NonBindingMatch) => Definite(old),
            (Definite(NonBindingMatch), new) => Definite(new),

            (Definite(old), CopyingMatch) => Definite(old),
            (Definite(CopyingMatch), new) => Definite(new),

            (Definite(_), _) => Conflicting,
            (Conflicting, _) => *self,
        };
    }

    fn match_mode(&self) -> MatchMode {
        match *self {
            Unknown => NonBindingMatch,
            Definite(mode) => mode,
            Conflicting => {
                // Conservatively return MovingMatch to let the
                // compiler continue to make progress.
                MovingMatch
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MutateMode {
    Init,
    JustWrite,    // x = y
    WriteAndRead, // x += y
}

#[derive(Copy, Clone)]
enum OverloadedCallType {
    FnOverloadedCall,
    FnMutOverloadedCall,
    FnOnceOverloadedCall,
}

impl OverloadedCallType {
    fn from_trait_id(tcx: TyCtxt, trait_id: DefId) -> OverloadedCallType {
        for &(maybe_function_trait, overloaded_call_type) in &[
            (tcx.lang_items().fn_once_trait(), FnOnceOverloadedCall),
            (tcx.lang_items().fn_mut_trait(), FnMutOverloadedCall),
            (tcx.lang_items().fn_trait(), FnOverloadedCall)
        ] {
            match maybe_function_trait {
                Some(function_trait) if function_trait == trait_id => {
                    return overloaded_call_type
                }
                _ => continue,
            }
        }

        bug!("overloaded call didn't map to known function trait")
    }

    fn from_method_id(tcx: TyCtxt, method_id: DefId) -> OverloadedCallType {
        let method = tcx.associated_item(method_id);
        OverloadedCallType::from_trait_id(tcx, method.container.id())
    }
}

///////////////////////////////////////////////////////////////////////////
// The ExprUseVisitor type
//
// This is the code that actually walks the tree.
pub struct ExprUseVisitor<'a, 'gcx: 'a+'tcx, 'tcx: 'a> {
    mc: mc::MemCategorizationContext<'a, 'gcx, 'tcx>,
    delegate: &'a mut dyn Delegate<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
}

// If the MC results in an error, it's because the type check
// failed (or will fail, when the error is uncovered and reported
// during writeback). In this case, we just ignore this part of the
// code.
//
// Note that this macro appears similar to try!(), but, unlike try!(),
// it does not propagate the error.
macro_rules! return_if_err {
    ($inp: expr) => (
        match $inp {
            Ok(v) => v,
            Err(()) => {
                debug!("mc reported err");
                return
            }
        }
    )
}

impl<'a, 'tcx> ExprUseVisitor<'a, 'tcx, 'tcx> {
    /// Creates the ExprUseVisitor, configuring it with the various options provided:
    ///
    /// - `delegate` -- who receives the callbacks
    /// - `param_env` --- parameter environment for trait lookups (esp. pertaining to `Copy`)
    /// - `region_scope_tree` --- region scope tree for the code being analyzed
    /// - `tables` --- typeck results for the code being analyzed
    /// - `rvalue_promotable_map` --- if you care about rvalue promotion, then provide
    ///   the map here (it can be computed with `tcx.rvalue_promotable_map(def_id)`).
    ///   `None` means that rvalues will be given more conservative lifetimes.
    ///
    /// See also `with_infer`, which is used *during* typeck.
    pub fn new(delegate: &'a mut (dyn Delegate<'tcx>+'a),
               tcx: TyCtxt<'a, 'tcx, 'tcx>,
               param_env: ty::ParamEnv<'tcx>,
               region_scope_tree: &'a region::ScopeTree,
               tables: &'a ty::TypeckTables<'tcx>,
               rvalue_promotable_map: Option<Lrc<ItemLocalSet>>)
               -> Self
    {
        ExprUseVisitor {
            mc: mc::MemCategorizationContext::new(tcx,
                                                  region_scope_tree,
                                                  tables,
                                                  rvalue_promotable_map),
            delegate,
            param_env,
        }
    }
}

impl<'a, 'gcx, 'tcx> ExprUseVisitor<'a, 'gcx, 'tcx> {
    pub fn with_infer(delegate: &'a mut (dyn Delegate<'tcx>+'a),
                      infcx: &'a InferCtxt<'a, 'gcx, 'tcx>,
                      param_env: ty::ParamEnv<'tcx>,
                      region_scope_tree: &'a region::ScopeTree,
                      tables: &'a ty::TypeckTables<'tcx>)
                      -> Self
    {
        ExprUseVisitor {
            mc: mc::MemCategorizationContext::with_infer(infcx, region_scope_tree, tables),
            delegate,
            param_env,
        }
    }

    pub fn consume_body(&mut self, body: &hir::Body) {
        debug!("consume_body(body={:?})", body);

        for arg in &body.arguments {
            let arg_ty = return_if_err!(self.mc.pat_ty_adjusted(&arg.pat));
            debug!("consume_body: arg_ty = {:?}", arg_ty);

            let fn_body_scope_r =
                self.tcx().mk_region(ty::ReScope(region::Scope::Node(body.value.hir_id.local_id)));
            let arg_cmt = Rc::new(self.mc.cat_rvalue(
                arg.id,
                arg.pat.span,
                fn_body_scope_r, // Args live only as long as the fn body.
                arg_ty));

            self.walk_irrefutable_pat(arg_cmt, &arg.pat);
        }

        self.consume_expr(&body.value);
    }

    fn tcx(&self) -> TyCtxt<'a, 'gcx, 'tcx> {
        self.mc.tcx
    }

    fn delegate_consume(&mut self,
                        consume_id: ast::NodeId,
                        consume_span: Span,
                        cmt: &mc::cmt_<'tcx>) {
        debug!("delegate_consume(consume_id={}, cmt={:?})",
               consume_id, cmt);

        let mode = copy_or_move(&self.mc, self.param_env, cmt, DirectRefMove);
        self.delegate.consume(consume_id, consume_span, cmt, mode);
    }

    fn consume_exprs(&mut self, exprs: &[hir::Expr]) {
        for expr in exprs {
            self.consume_expr(&expr);
        }
    }

    pub fn consume_expr(&mut self, expr: &hir::Expr) {
        debug!("consume_expr(expr={:?})", expr);

        let cmt = return_if_err!(self.mc.cat_expr(expr));
        self.delegate_consume(expr.id, expr.span, &cmt);
        self.walk_expr(expr);
    }

    fn mutate_expr(&mut self,
                   assignment_expr: &hir::Expr,
                   expr: &hir::Expr,
                   mode: MutateMode) {
        let cmt = return_if_err!(self.mc.cat_expr(expr));
        self.delegate.mutate(assignment_expr.id, assignment_expr.span, &cmt, mode);
        self.walk_expr(expr);
    }

    fn borrow_expr(&mut self,
                   expr: &hir::Expr,
                   r: ty::Region<'tcx>,
                   bk: ty::BorrowKind,
                   cause: LoanCause) {
        debug!("borrow_expr(expr={:?}, r={:?}, bk={:?})",
               expr, r, bk);

        let cmt = return_if_err!(self.mc.cat_expr(expr));
        self.delegate.borrow(expr.id, expr.span, &cmt, r, bk, cause);

        self.walk_expr(expr)
    }

    fn select_from_expr(&mut self, expr: &hir::Expr) {
        self.walk_expr(expr)
    }

    pub fn walk_expr(&mut self, expr: &hir::Expr) {
        debug!("walk_expr(expr={:?})", expr);

        self.walk_adjustment(expr);

        match expr.node {
            hir::ExprPath(_) => { }

            hir::ExprType(ref subexpr, _) => {
                self.walk_expr(&subexpr)
            }

            hir::ExprUnary(hir::UnDeref, ref base) => {      // *base
                self.select_from_expr(&base);
            }

            hir::ExprField(ref base, _) => {         // base.f
                self.select_from_expr(&base);
            }

            hir::ExprIndex(ref lhs, ref rhs) => {       // lhs[rhs]
                self.select_from_expr(&lhs);
                self.consume_expr(&rhs);
            }

            hir::ExprCall(ref callee, ref args) => {    // callee(args)
                self.walk_callee(expr, &callee);
                self.consume_exprs(args);
            }

            hir::ExprMethodCall(.., ref args) => { // callee.m(args)
                self.consume_exprs(args);
            }

            hir::ExprStruct(_, ref fields, ref opt_with) => {
                self.walk_struct_expr(fields, opt_with);
            }

            hir::ExprTup(ref exprs) => {
                self.consume_exprs(exprs);
            }

            hir::ExprIf(ref cond_expr, ref then_expr, ref opt_else_expr) => {
                self.consume_expr(&cond_expr);
                self.walk_expr(&then_expr);
                if let Some(ref else_expr) = *opt_else_expr {
                    self.consume_expr(&else_expr);
                }
            }

            hir::ExprMatch(ref discr, ref arms, _) => {
                let discr_cmt = Rc::new(return_if_err!(self.mc.cat_expr(&discr)));
                let r = self.tcx().types.re_empty;
                self.borrow_expr(&discr, r, ty::ImmBorrow, MatchDiscriminant);

                // treatment of the discriminant is handled while walking the arms.
                for arm in arms {
                    let mode = self.arm_move_mode(discr_cmt.clone(), arm);
                    let mode = mode.match_mode();
                    self.walk_arm(discr_cmt.clone(), arm, mode);
                }
            }

            hir::ExprArray(ref exprs) => {
                self.consume_exprs(exprs);
            }

            hir::ExprAddrOf(m, ref base) => {   // &base
                // make sure that the thing we are pointing out stays valid
                // for the lifetime `scope_r` of the resulting ptr:
                let expr_ty = return_if_err!(self.mc.expr_ty(expr));
                if let ty::TyRef(r, _, _) = expr_ty.sty {
                    let bk = ty::BorrowKind::from_mutbl(m);
                    self.borrow_expr(&base, r, bk, AddrOf);
                }
            }

            hir::ExprInlineAsm(ref ia, ref outputs, ref inputs) => {
                for (o, output) in ia.outputs.iter().zip(outputs) {
                    if o.is_indirect {
                        self.consume_expr(output);
                    } else {
                        self.mutate_expr(expr, output,
                                         if o.is_rw {
                                             MutateMode::WriteAndRead
                                         } else {
                                             MutateMode::JustWrite
                                         });
                    }
                }
                self.consume_exprs(inputs);
            }

            hir::ExprAgain(..) |
            hir::ExprLit(..) => {}

            hir::ExprLoop(ref blk, _, _) => {
                self.walk_block(&blk);
            }

            hir::ExprWhile(ref cond_expr, ref blk, _) => {
                self.consume_expr(&cond_expr);
                self.walk_block(&blk);
            }

            hir::ExprUnary(_, ref lhs) => {
                self.consume_expr(&lhs);
            }

            hir::ExprBinary(_, ref lhs, ref rhs) => {
                self.consume_expr(&lhs);
                self.consume_expr(&rhs);
            }

            hir::ExprBlock(ref blk, _) => {
                self.walk_block(&blk);
            }

            hir::ExprBreak(_, ref opt_expr) | hir::ExprRet(ref opt_expr) => {
                if let Some(ref expr) = *opt_expr {
                    self.consume_expr(&expr);
                }
            }

            hir::ExprAssign(ref lhs, ref rhs) => {
                self.mutate_expr(expr, &lhs, MutateMode::JustWrite);
                self.consume_expr(&rhs);
            }

            hir::ExprCast(ref base, _) => {
                self.consume_expr(&base);
            }

            hir::ExprAssignOp(_, ref lhs, ref rhs) => {
                if self.mc.tables.is_method_call(expr) {
                    self.consume_expr(lhs);
                } else {
                    self.mutate_expr(expr, &lhs, MutateMode::WriteAndRead);
                }
                self.consume_expr(&rhs);
            }

            hir::ExprRepeat(ref base, _) => {
                self.consume_expr(&base);
            }

            hir::ExprClosure(.., fn_decl_span, _) => {
                self.walk_captures(expr, fn_decl_span)
            }

            hir::ExprBox(ref base) => {
                self.consume_expr(&base);
            }

            hir::ExprYield(ref value) => {
                self.consume_expr(&value);
            }
        }
    }

    fn walk_callee(&mut self, call: &hir::Expr, callee: &hir::Expr) {
        let callee_ty = return_if_err!(self.mc.expr_ty_adjusted(callee));
        debug!("walk_callee: callee={:?} callee_ty={:?}",
               callee, callee_ty);
        match callee_ty.sty {
            ty::TyFnDef(..) | ty::TyFnPtr(_) => {
                self.consume_expr(callee);
            }
            ty::TyError => { }
            _ => {
                if let Some(def) = self.mc.tables.type_dependent_defs().get(call.hir_id) {
                    let def_id = def.def_id();
                    let call_scope = region::Scope::Node(call.hir_id.local_id);
                    match OverloadedCallType::from_method_id(self.tcx(), def_id) {
                        FnMutOverloadedCall => {
                            let call_scope_r = self.tcx().mk_region(ty::ReScope(call_scope));
                            self.borrow_expr(callee,
                                            call_scope_r,
                                            ty::MutBorrow,
                                            ClosureInvocation);
                        }
                        FnOverloadedCall => {
                            let call_scope_r = self.tcx().mk_region(ty::ReScope(call_scope));
                            self.borrow_expr(callee,
                                            call_scope_r,
                                            ty::ImmBorrow,
                                            ClosureInvocation);
                        }
                        FnOnceOverloadedCall => self.consume_expr(callee),
                    }
                } else {
                    self.tcx().sess.delay_span_bug(call.span,
                                                   "no type-dependent def for overloaded call");
                }
            }
        }
    }

    fn walk_stmt(&mut self, stmt: &hir::Stmt) {
        match stmt.node {
            hir::StmtDecl(ref decl, _) => {
                match decl.node {
                    hir::DeclLocal(ref local) => {
                        self.walk_local(&local);
                    }

                    hir::DeclItem(_) => {
                        // we don't visit nested items in this visitor,
                        // only the fn body we were given.
                    }
                }
            }

            hir::StmtExpr(ref expr, _) |
            hir::StmtSemi(ref expr, _) => {
                self.consume_expr(&expr);
            }
        }
    }

    fn walk_local(&mut self, local: &hir::Local) {
        match local.init {
            None => {
                local.pat.each_binding(|_, hir_id, span, _| {
                    let node_id = self.mc.tcx.hir.hir_to_node_id(hir_id);
                    self.delegate.decl_without_init(node_id, span);
                })
            }

            Some(ref expr) => {
                // Variable declarations with
                // initializers are considered
                // "assigns", which is handled by
                // `walk_pat`:
                self.walk_expr(&expr);
                let init_cmt = Rc::new(return_if_err!(self.mc.cat_expr(&expr)));
                self.walk_irrefutable_pat(init_cmt, &local.pat);
            }
        }
    }

    /// Indicates that the value of `blk` will be consumed, meaning either copied or moved
    /// depending on its type.
    fn walk_block(&mut self, blk: &hir::Block) {
        debug!("walk_block(blk.id={})", blk.id);

        for stmt in &blk.stmts {
            self.walk_stmt(stmt);
        }

        if let Some(ref tail_expr) = blk.expr {
            self.consume_expr(&tail_expr);
        }
    }

    fn walk_struct_expr(&mut self,
                        fields: &[hir::Field],
                        opt_with: &Option<P<hir::Expr>>) {
        // Consume the expressions supplying values for each field.
        for field in fields {
            self.consume_expr(&field.expr);
        }

        let with_expr = match *opt_with {
            Some(ref w) => &**w,
            None => { return; }
        };

        let with_cmt = Rc::new(return_if_err!(self.mc.cat_expr(&with_expr)));

        // Select just those fields of the `with`
        // expression that will actually be used
        match with_cmt.ty.sty {
            ty::TyAdt(adt, substs) if adt.is_struct() => {
                // Consume those fields of the with expression that are needed.
                for (f_index, with_field) in adt.non_enum_variant().fields.iter().enumerate() {
                    let is_mentioned = fields.iter().any(|f| {
                        self.tcx().field_index(f.id, self.mc.tables) == f_index
                    });
                    if !is_mentioned {
                        let cmt_field = self.mc.cat_field(
                            &*with_expr,
                            with_cmt.clone(),
                            f_index,
                            with_field.ident,
                            with_field.ty(self.tcx(), substs)
                        );
                        self.delegate_consume(with_expr.id, with_expr.span, &cmt_field);
                    }
                }
            }
            _ => {
                // the base expression should always evaluate to a
                // struct; however, when EUV is run during typeck, it
                // may not. This will generate an error earlier in typeck,
                // so we can just ignore it.
                if !self.tcx().sess.has_errors() {
                    span_bug!(
                        with_expr.span,
                        "with expression doesn't evaluate to a struct");
                }
            }
        }

        // walk the with expression so that complex expressions
        // are properly handled.
        self.walk_expr(with_expr);
    }

    // Invoke the appropriate delegate calls for anything that gets
    // consumed or borrowed as part of the automatic adjustment
    // process.
    fn walk_adjustment(&mut self, expr: &hir::Expr) {
        let adjustments = self.mc.tables.expr_adjustments(expr);
        let mut cmt = return_if_err!(self.mc.cat_expr_unadjusted(expr));
        for adjustment in adjustments {
            debug!("walk_adjustment expr={:?} adj={:?}", expr, adjustment);
            match adjustment.kind {
                adjustment::Adjust::NeverToAny |
                adjustment::Adjust::ReifyFnPointer |
                adjustment::Adjust::UnsafeFnPointer |
                adjustment::Adjust::ClosureFnPointer |
                adjustment::Adjust::MutToConstPointer |
                adjustment::Adjust::Unsize => {
                    // Creating a closure/fn-pointer or unsizing consumes
                    // the input and stores it into the resulting rvalue.
                    self.delegate_consume(expr.id, expr.span, &cmt);
                }

                adjustment::Adjust::Deref(None) => {}

                // Autoderefs for overloaded Deref calls in fact reference
                // their receiver. That is, if we have `(*x)` where `x`
                // is of type `Rc<T>`, then this in fact is equivalent to
                // `x.deref()`. Since `deref()` is declared with `&self`,
                // this is an autoref of `x`.
                adjustment::Adjust::Deref(Some(ref deref)) => {
                    let bk = ty::BorrowKind::from_mutbl(deref.mutbl);
                    self.delegate.borrow(expr.id, expr.span, &cmt, deref.region, bk, AutoRef);
                }

                adjustment::Adjust::Borrow(ref autoref) => {
                    self.walk_autoref(expr, &cmt, autoref);
                }
            }
            cmt = return_if_err!(self.mc.cat_expr_adjusted(expr, cmt, &adjustment));
        }
    }

    /// Walks the autoref `autoref` applied to the autoderef'd
    /// `expr`. `cmt_base` is the mem-categorized form of `expr`
    /// after all relevant autoderefs have occurred.
    fn walk_autoref(&mut self,
                    expr: &hir::Expr,
                    cmt_base: &mc::cmt_<'tcx>,
                    autoref: &adjustment::AutoBorrow<'tcx>) {
        debug!("walk_autoref(expr.id={} cmt_base={:?} autoref={:?})",
               expr.id,
               cmt_base,
               autoref);

        match *autoref {
            adjustment::AutoBorrow::Ref(r, m) => {
                self.delegate.borrow(expr.id,
                                     expr.span,
                                     cmt_base,
                                     r,
                                     ty::BorrowKind::from_mutbl(m.into()),
                                     AutoRef);
            }

            adjustment::AutoBorrow::RawPtr(m) => {
                debug!("walk_autoref: expr.id={} cmt_base={:?}",
                       expr.id,
                       cmt_base);

                // Converting from a &T to *T (or &mut T to *mut T) is
                // treated as borrowing it for the enclosing temporary
                // scope.
                let r = self.tcx().mk_region(ty::ReScope(
                    region::Scope::Node(expr.hir_id.local_id)));

                self.delegate.borrow(expr.id,
                                     expr.span,
                                     cmt_base,
                                     r,
                                     ty::BorrowKind::from_mutbl(m),
                                     AutoUnsafe);
            }
        }
    }

    fn arm_move_mode(&mut self, discr_cmt: mc::cmt<'tcx>, arm: &hir::Arm) -> TrackMatchMode {
        let mut mode = Unknown;
        for pat in &arm.pats {
            self.determine_pat_move_mode(discr_cmt.clone(), &pat, &mut mode);
        }
        mode
    }

    fn walk_arm(&mut self, discr_cmt: mc::cmt<'tcx>, arm: &hir::Arm, mode: MatchMode) {
        for pat in &arm.pats {
            self.walk_pat(discr_cmt.clone(), &pat, mode);
        }

        if let Some(ref guard) = arm.guard {
            self.consume_expr(&guard);
        }

        self.consume_expr(&arm.body);
    }

    /// Walks a pat that occurs in isolation (i.e. top-level of fn
    /// arg or let binding.  *Not* a match arm or nested pat.)
    fn walk_irrefutable_pat(&mut self, cmt_discr: mc::cmt<'tcx>, pat: &hir::Pat) {
        let mut mode = Unknown;
        self.determine_pat_move_mode(cmt_discr.clone(), pat, &mut mode);
        let mode = mode.match_mode();
        self.walk_pat(cmt_discr, pat, mode);
    }

    /// Identifies any bindings within `pat` and accumulates within
    /// `mode` whether the overall pattern/match structure is a move,
    /// copy, or borrow.
    fn determine_pat_move_mode(&mut self,
                               cmt_discr: mc::cmt<'tcx>,
                               pat: &hir::Pat,
                               mode: &mut TrackMatchMode) {
        debug!("determine_pat_move_mode cmt_discr={:?} pat={:?}", cmt_discr,
               pat);
        return_if_err!(self.mc.cat_pattern(cmt_discr, pat, |cmt_pat, pat| {
            if let PatKind::Binding(..) = pat.node {
                let bm = *self.mc.tables.pat_binding_modes().get(pat.hir_id)
                                                          .expect("missing binding mode");
                match bm {
                    ty::BindByReference(..) =>
                        mode.lub(BorrowingMatch),
                    ty::BindByValue(..) => {
                        match copy_or_move(&self.mc, self.param_env, &cmt_pat, PatBindingMove) {
                            Copy => mode.lub(CopyingMatch),
                            Move(..) => mode.lub(MovingMatch),
                        }
                    }
                }
            }
        }));
    }

    /// The core driver for walking a pattern; `match_mode` must be
    /// established up front, e.g. via `determine_pat_move_mode` (see
    /// also `walk_irrefutable_pat` for patterns that stand alone).
    fn walk_pat(&mut self, cmt_discr: mc::cmt<'tcx>, pat: &hir::Pat, match_mode: MatchMode) {
        debug!("walk_pat(cmt_discr={:?}, pat={:?})", cmt_discr, pat);

        let ExprUseVisitor { ref mc, ref mut delegate, param_env } = *self;
        return_if_err!(mc.cat_pattern(cmt_discr.clone(), pat, |cmt_pat, pat| {
            if let PatKind::Binding(_, canonical_id, ..) = pat.node {
                debug!(
                    "walk_pat: binding cmt_pat={:?} pat={:?} match_mode={:?}",
                    cmt_pat,
                    pat,
                    match_mode,
                );
                let bm = *mc.tables.pat_binding_modes().get(pat.hir_id)
                                                     .expect("missing binding mode");
                debug!("walk_pat: pat.hir_id={:?} bm={:?}", pat.hir_id, bm);

                // pat_ty: the type of the binding being produced.
                let pat_ty = return_if_err!(mc.node_ty(pat.hir_id));
                debug!("walk_pat: pat_ty={:?}", pat_ty);

                // Each match binding is effectively an assignment to the
                // binding being produced.
                let def = Def::Local(canonical_id);
                if let Ok(ref binding_cmt) = mc.cat_def(pat.id, pat.span, pat_ty, def) {
                    delegate.mutate(pat.id, pat.span, binding_cmt, MutateMode::Init);
                }

                // It is also a borrow or copy/move of the value being matched.
                match bm {
                    ty::BindByReference(m) => {
                        if let ty::TyRef(r, _, _) = pat_ty.sty {
                            let bk = ty::BorrowKind::from_mutbl(m);
                            delegate.borrow(pat.id, pat.span, &cmt_pat, r, bk, RefBinding);
                        }
                    }
                    ty::BindByValue(..) => {
                        let mode = copy_or_move(mc, param_env, &cmt_pat, PatBindingMove);
                        debug!("walk_pat binding consuming pat");
                        delegate.consume_pat(pat, &cmt_pat, mode);
                    }
                }
            }
        }));

        // Do a second pass over the pattern, calling `matched_pat` on
        // the interior nodes (enum variants and structs), as opposed
        // to the above loop's visit of than the bindings that form
        // the leaves of the pattern tree structure.
        return_if_err!(mc.cat_pattern(cmt_discr, pat, |cmt_pat, pat| {
            let qpath = match pat.node {
                PatKind::Path(ref qpath) |
                PatKind::TupleStruct(ref qpath, ..) |
                PatKind::Struct(ref qpath, ..) => qpath,
                _ => return
            };
            let def = mc.tables.qpath_def(qpath, pat.hir_id);
            match def {
                Def::Variant(variant_did) |
                Def::VariantCtor(variant_did, ..) => {
                    let downcast_cmt = mc.cat_downcast_if_needed(pat, cmt_pat, variant_did);

                    debug!("variant downcast_cmt={:?} pat={:?}", downcast_cmt, pat);
                    delegate.matched_pat(pat, &downcast_cmt, match_mode);
                }
                Def::Struct(..) | Def::StructCtor(..) | Def::Union(..) |
                Def::TyAlias(..) | Def::AssociatedTy(..) | Def::SelfTy(..) => {
                    debug!("struct cmt_pat={:?} pat={:?}", cmt_pat, pat);
                    delegate.matched_pat(pat, &cmt_pat, match_mode);
                }
                _ => {}
            }
        }));
    }

    fn walk_captures(&mut self, closure_expr: &hir::Expr, fn_decl_span: Span) {
        debug!("walk_captures({:?})", closure_expr);

        self.tcx().with_freevars(closure_expr.id, |freevars| {
            for freevar in freevars {
                let var_hir_id = self.tcx().hir.node_to_hir_id(freevar.var_id());
                let closure_def_id = self.tcx().hir.local_def_id(closure_expr.id);
                let upvar_id = ty::UpvarId {
                    var_id: var_hir_id,
                    closure_expr_id: closure_def_id.to_local(),
                };
                let upvar_capture = self.mc.tables.upvar_capture(upvar_id);
                let cmt_var = return_if_err!(self.cat_captured_var(closure_expr.id,
                                                                   fn_decl_span,
                                                                   freevar));
                match upvar_capture {
                    ty::UpvarCapture::ByValue => {
                        let mode = copy_or_move(&self.mc,
                                                self.param_env,
                                                &cmt_var,
                                                CaptureMove);
                        self.delegate.consume(closure_expr.id, freevar.span, &cmt_var, mode);
                    }
                    ty::UpvarCapture::ByRef(upvar_borrow) => {
                        self.delegate.borrow(closure_expr.id,
                                             fn_decl_span,
                                             &cmt_var,
                                             upvar_borrow.region,
                                             upvar_borrow.kind,
                                             ClosureCapture(freevar.span));
                    }
                }
            }
        });
    }

    fn cat_captured_var(&mut self,
                        closure_id: ast::NodeId,
                        closure_span: Span,
                        upvar: &hir::Freevar)
                        -> mc::McResult<mc::cmt_<'tcx>> {
        // Create the cmt for the variable being borrowed, from the
        // caller's perspective
        let var_hir_id = self.tcx().hir.node_to_hir_id(upvar.var_id());
        let var_ty = self.mc.node_ty(var_hir_id)?;
        self.mc.cat_def(closure_id, closure_span, var_ty, upvar.def)
    }
}

fn copy_or_move<'a, 'gcx, 'tcx>(mc: &mc::MemCategorizationContext<'a, 'gcx, 'tcx>,
                                param_env: ty::ParamEnv<'tcx>,
                                cmt: &mc::cmt_<'tcx>,
                                move_reason: MoveReason)
                                -> ConsumeMode
{
    if mc.type_moves_by_default(param_env, cmt.ty, cmt.span) {
        Move(move_reason)
    } else {
        Copy
    }
}
