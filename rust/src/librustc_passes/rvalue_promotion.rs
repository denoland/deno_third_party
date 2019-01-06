// Verifies that the types and values of const and static items
// are safe. The rules enforced by this module are:
//
// - For each *mutable* static item, it checks that its **type**:
//     - doesn't have a destructor
//     - doesn't own a box
//
// - For each *immutable* static item, it checks that its **value**:
//       - doesn't own a box
//       - doesn't contain a struct literal or a call to an enum variant / struct constructor where
//           - the type of the struct/enum has a dtor
//
// Rules Enforced Elsewhere:
// - It's not possible to take the address of a static item with unsafe interior. This is enforced
// by borrowck::gather_loans

use rustc::ty::cast::CastKind;
use rustc::hir::def::{Def, CtorKind};
use rustc::hir::def_id::DefId;
use rustc::middle::expr_use_visitor as euv;
use rustc::middle::mem_categorization as mc;
use rustc::middle::mem_categorization::Categorization;
use rustc::ty::{self, Ty, TyCtxt};
use rustc::ty::query::Providers;
use rustc::ty::subst::Substs;
use rustc::util::nodemap::{ItemLocalSet, NodeSet};
use rustc::hir;
use rustc_data_structures::sync::Lrc;
use syntax::ast;
use syntax_pos::{Span, DUMMY_SP};
use self::Promotability::*;
use std::ops::{BitAnd, BitAndAssign, BitOr};

pub fn provide(providers: &mut Providers) {
    *providers = Providers {
        rvalue_promotable_map,
        const_is_rvalue_promotable_to_static,
        ..*providers
    };
}

pub fn check_crate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>) {
    for &body_id in &tcx.hir().krate().body_ids {
        let def_id = tcx.hir().body_owner_def_id(body_id);
        tcx.const_is_rvalue_promotable_to_static(def_id);
    }
    tcx.sess.abort_if_errors();
}

fn const_is_rvalue_promotable_to_static<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                                  def_id: DefId)
                                                  -> bool
{
    assert!(def_id.is_local());

    let node_id = tcx.hir().as_local_node_id(def_id)
        .expect("rvalue_promotable_map invoked with non-local def-id");
    let body_id = tcx.hir().body_owned_by(node_id);
    let body_hir_id = tcx.hir().node_to_hir_id(body_id.node_id);
    tcx.rvalue_promotable_map(def_id).contains(&body_hir_id.local_id)
}

fn rvalue_promotable_map<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                   def_id: DefId)
                                   -> Lrc<ItemLocalSet>
{
    let outer_def_id = tcx.closure_base_def_id(def_id);
    if outer_def_id != def_id {
        return tcx.rvalue_promotable_map(outer_def_id);
    }

    let mut visitor = CheckCrateVisitor {
        tcx,
        tables: &ty::TypeckTables::empty(None),
        in_fn: false,
        in_static: false,
        mut_rvalue_borrows: Default::default(),
        param_env: ty::ParamEnv::empty(),
        identity_substs: Substs::empty(),
        result: ItemLocalSet::default(),
    };

    // `def_id` should be a `Body` owner
    let node_id = tcx.hir().as_local_node_id(def_id)
        .expect("rvalue_promotable_map invoked with non-local def-id");
    let body_id = tcx.hir().body_owned_by(node_id);
    let _ = visitor.check_nested_body(body_id);

    Lrc::new(visitor.result)
}

struct CheckCrateVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    in_fn: bool,
    in_static: bool,
    mut_rvalue_borrows: NodeSet,
    param_env: ty::ParamEnv<'tcx>,
    identity_substs: &'tcx Substs<'tcx>,
    tables: &'a ty::TypeckTables<'tcx>,
    result: ItemLocalSet,
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Promotability {
    Promotable,
    NotPromotable
}

impl BitAnd for Promotability {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Promotable, Promotable) => Promotable,
            _ => NotPromotable,
        }
    }
}

impl BitAndAssign for Promotability {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs
    }
}

impl BitOr for Promotability {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        match (self, rhs) {
            (NotPromotable, NotPromotable) => NotPromotable,
            _ => Promotable,
        }
    }
}

impl<'a, 'gcx> CheckCrateVisitor<'a, 'gcx> {
    // Returns true iff all the values of the type are promotable.
    fn type_promotability(&mut self, ty: Ty<'gcx>) -> Promotability {
        debug!("type_promotability({})", ty);

        if ty.is_freeze(self.tcx, self.param_env, DUMMY_SP) &&
            !ty.needs_drop(self.tcx, self.param_env) {
            Promotable
        } else {
            NotPromotable
        }
    }

    fn handle_const_fn_call(
        &mut self,
        def_id: DefId,
    ) -> Promotability {
        if self.tcx.is_promotable_const_fn(def_id) {
            Promotable
        } else {
            NotPromotable
        }
    }

    /// While the `ExprUseVisitor` walks, we will identify which
    /// expressions are borrowed, and insert their ids into this
    /// table. Actually, we insert the "borrow-id", which is normally
    /// the id of the expression being borrowed: but in the case of
    /// `ref mut` borrows, the `id` of the pattern is
    /// inserted. Therefore later we remove that entry from the table
    /// and transfer it over to the value being matched. This will
    /// then prevent said value from being promoted.
    fn remove_mut_rvalue_borrow(&mut self, pat: &hir::Pat) -> bool {
        let mut any_removed = false;
        pat.walk(|p| {
            any_removed |= self.mut_rvalue_borrows.remove(&p.id);
            true
        });
        any_removed
    }
}

impl<'a, 'tcx> CheckCrateVisitor<'a, 'tcx> {
    fn check_nested_body(&mut self, body_id: hir::BodyId) -> Promotability {
        let item_id = self.tcx.hir().body_owner(body_id);
        let item_def_id = self.tcx.hir().local_def_id(item_id);

        let outer_in_fn = self.in_fn;
        let outer_tables = self.tables;
        let outer_param_env = self.param_env;
        let outer_identity_substs = self.identity_substs;

        self.in_fn = false;
        self.in_static = false;

        match self.tcx.hir().body_owner_kind(item_id) {
            hir::BodyOwnerKind::Fn => self.in_fn = true,
            hir::BodyOwnerKind::Static(_) => self.in_static = true,
            _ => {}
        };


        self.tables = self.tcx.typeck_tables_of(item_def_id);
        self.param_env = self.tcx.param_env(item_def_id);
        self.identity_substs = Substs::identity_for_item(self.tcx, item_def_id);

        let body = self.tcx.hir().body(body_id);

        let tcx = self.tcx;
        let param_env = self.param_env;
        let region_scope_tree = self.tcx.region_scope_tree(item_def_id);
        let tables = self.tables;
        euv::ExprUseVisitor::new(self, tcx, param_env, &region_scope_tree, tables, None)
            .consume_body(body);

        let body_promotable = self.check_expr(&body.value);
        self.in_fn = outer_in_fn;
        self.tables = outer_tables;
        self.param_env = outer_param_env;
        self.identity_substs = outer_identity_substs;
        body_promotable
    }

    fn check_stmt(&mut self, stmt: &'tcx hir::Stmt) -> Promotability {
        match stmt.node {
            hir::StmtKind::Decl(ref decl, _node_id) => {
                match &decl.node {
                    hir::DeclKind::Local(local) => {
                        if self.remove_mut_rvalue_borrow(&local.pat) {
                            if let Some(init) = &local.init {
                                self.mut_rvalue_borrows.insert(init.id);
                            }
                        }

                        if let Some(ref expr) = local.init {
                            let _ = self.check_expr(&expr);
                        }
                        NotPromotable
                    }
                    // Item statements are allowed
                    hir::DeclKind::Item(_) => Promotable
                }
            }
            hir::StmtKind::Expr(ref box_expr, _node_id) |
            hir::StmtKind::Semi(ref box_expr, _node_id) => {
                let _ = self.check_expr(box_expr);
                NotPromotable
            }
        }
    }

    fn check_expr(&mut self, ex: &'tcx hir::Expr) -> Promotability {
        let node_ty = self.tables.node_id_to_type(ex.hir_id);
        let mut outer = check_expr_kind(self, ex, node_ty);
        outer &= check_adjustments(self, ex);

        // Handle borrows on (or inside the autorefs of) this expression.
        if self.mut_rvalue_borrows.remove(&ex.id) {
            outer = NotPromotable
        }

        if outer == Promotable {
            self.result.insert(ex.hir_id.local_id);
        }
        outer
    }

    fn check_block(&mut self, block: &'tcx hir::Block) -> Promotability {
        let mut iter_result = Promotable;
        for index in block.stmts.iter() {
            iter_result &= self.check_stmt(index);
        }
        match block.expr {
            Some(ref box_expr) => iter_result & self.check_expr(&*box_expr),
            None => iter_result,
        }
    }
}

/// This function is used to enforce the constraints on
/// const/static items. It walks through the *value*
/// of the item walking down the expression and evaluating
/// every nested expression. If the expression is not part
/// of a const/static item, it is qualified for promotion
/// instead of producing errors.
fn check_expr_kind<'a, 'tcx>(
    v: &mut CheckCrateVisitor<'a, 'tcx>,
    e: &'tcx hir::Expr, node_ty: Ty<'tcx>) -> Promotability {

    let ty_result = match node_ty.sty {
        ty::Adt(def, _) if def.has_dtor(v.tcx) => {
            NotPromotable
        }
        _ => Promotable
    };

    let node_result = match e.node {
        hir::ExprKind::Box(ref expr) => {
            let _ = v.check_expr(&expr);
            NotPromotable
        }
        hir::ExprKind::Unary(op, ref expr) => {
            let expr_promotability = v.check_expr(expr);
            if v.tables.is_method_call(e) || op == hir::UnDeref {
                return NotPromotable;
            }
            expr_promotability
        }
        hir::ExprKind::Binary(op, ref lhs, ref rhs) => {
            let lefty = v.check_expr(lhs);
            let righty = v.check_expr(rhs);
            if v.tables.is_method_call(e) {
                return NotPromotable;
            }
            match v.tables.node_id_to_type(lhs.hir_id).sty {
                ty::RawPtr(_) | ty::FnPtr(..) => {
                    assert!(op.node == hir::BinOpKind::Eq || op.node == hir::BinOpKind::Ne ||
                            op.node == hir::BinOpKind::Le || op.node == hir::BinOpKind::Lt ||
                            op.node == hir::BinOpKind::Ge || op.node == hir::BinOpKind::Gt);

                    NotPromotable
                }
                _ => lefty & righty
            }
        }
        hir::ExprKind::Cast(ref from, _) => {
            let expr_promotability = v.check_expr(from);
            debug!("Checking const cast(id={})", from.id);
            match v.tables.cast_kinds().get(from.hir_id) {
                None => {
                    v.tcx.sess.delay_span_bug(e.span, "no kind for cast");
                    NotPromotable
                },
                Some(&CastKind::PtrAddrCast) | Some(&CastKind::FnPtrAddrCast) => {
                    NotPromotable
                }
                _ => expr_promotability
            }
        }
        hir::ExprKind::Path(ref qpath) => {
            let def = v.tables.qpath_def(qpath, e.hir_id);
            match def {
                Def::VariantCtor(..) | Def::StructCtor(..) |
                Def::Fn(..) | Def::Method(..) | Def::SelfCtor(..) => Promotable,

                // References to a static that are themselves within a static
                // are inherently promotable with the exception
                //  of "#[thread_local]" statics, which may not
                // outlive the current function
                Def::Static(did, _) => {

                    if v.in_static {
                        for attr in &v.tcx.get_attrs(did)[..] {
                            if attr.check_name("thread_local") {
                                debug!("Reference to Static(id={:?}) is unpromotable \
                                       due to a #[thread_local] attribute", did);
                                return NotPromotable;
                            }
                        }
                        Promotable
                    } else {
                        debug!("Reference to Static(id={:?}) is unpromotable as it is not \
                               referenced from a static", did);
                        NotPromotable
                    }
                }

                Def::Const(did) |
                Def::AssociatedConst(did) => {
                    let promotable = if v.tcx.trait_of_item(did).is_some() {
                        // Don't peek inside trait associated constants.
                        NotPromotable
                    } else if v.tcx.at(e.span).const_is_rvalue_promotable_to_static(did) {
                        Promotable
                    } else {
                        NotPromotable
                    };
                    // Just in case the type is more specific than the definition,
                    // e.g., impl associated const with type parameters, check it.
                    // Also, trait associated consts are relaxed by this.
                    promotable | v.type_promotability(node_ty)
                }
                _ => NotPromotable
            }
        }
        hir::ExprKind::Call(ref callee, ref hirvec) => {
            let mut call_result = v.check_expr(callee);
            for index in hirvec.iter() {
                call_result &= v.check_expr(index);
            }
            let mut callee = &**callee;
            loop {
                callee = match callee.node {
                    hir::ExprKind::Block(ref block, _) => match block.expr {
                        Some(ref tail) => &tail,
                        None => break
                    },
                    _ => break
                };
            }
            // The callee is an arbitrary expression, it doesn't necessarily have a definition.
            let def = if let hir::ExprKind::Path(ref qpath) = callee.node {
                v.tables.qpath_def(qpath, callee.hir_id)
            } else {
                Def::Err
            };
            let def_result = match def {
                Def::StructCtor(_, CtorKind::Fn) |
                Def::VariantCtor(_, CtorKind::Fn) |
                Def::SelfCtor(..) => Promotable,
                Def::Fn(did) => v.handle_const_fn_call(did),
                Def::Method(did) => {
                    match v.tcx.associated_item(did).container {
                        ty::ImplContainer(_) => v.handle_const_fn_call(did),
                        ty::TraitContainer(_) => NotPromotable,
                    }
                }
                _ => NotPromotable,
            };
            def_result & call_result
        }
        hir::ExprKind::MethodCall(ref _pathsegment, ref _span, ref hirvec) => {
            let mut method_call_result = Promotable;
            for index in hirvec.iter() {
                method_call_result &= v.check_expr(index);
            }
            if let Some(def) = v.tables.type_dependent_defs().get(e.hir_id) {
                let def_id = def.def_id();
                match v.tcx.associated_item(def_id).container {
                    ty::ImplContainer(_) => method_call_result & v.handle_const_fn_call(def_id),
                    ty::TraitContainer(_) => NotPromotable,
                }
            } else {
                v.tcx.sess.delay_span_bug(e.span, "no type-dependent def for method call");
                NotPromotable
            }
        }
        hir::ExprKind::Struct(ref _qpath, ref hirvec, ref option_expr) => {
            let mut struct_result = Promotable;
            for index in hirvec.iter() {
                struct_result &= v.check_expr(&index.expr);
            }
            if let Some(ref expr) = *option_expr {
                struct_result &= v.check_expr(&expr);
            }
            if let ty::Adt(adt, ..) = v.tables.expr_ty(e).sty {
                // unsafe_cell_type doesn't necessarily exist with no_core
                if Some(adt.did) == v.tcx.lang_items().unsafe_cell_type() {
                    return NotPromotable;
                }
            }
            struct_result
        }

        hir::ExprKind::Lit(_) |
        hir::ExprKind::Err => Promotable,

        hir::ExprKind::AddrOf(_, ref expr) |
        hir::ExprKind::Repeat(ref expr, _) => {
            v.check_expr(&expr)
        }

        hir::ExprKind::Closure(_capture_clause, ref _box_fn_decl,
                               body_id, _span, _option_generator_movability) => {
            let nested_body_promotable = v.check_nested_body(body_id);
            // Paths in constant contexts cannot refer to local variables,
            // as there are none, and thus closures can't have upvars there.
            if v.tcx.with_freevars(e.id, |fv| !fv.is_empty()) {
                NotPromotable
            } else {
                nested_body_promotable
            }
        }

        hir::ExprKind::Field(ref expr, _ident) => {
            let expr_promotability = v.check_expr(&expr);
            if let Some(def) = v.tables.expr_ty(expr).ty_adt_def() {
                if def.is_union() {
                    return NotPromotable;
                }
            }
            expr_promotability
        }

        hir::ExprKind::Block(ref box_block, ref _option_label) => {
            v.check_block(box_block)
        }

        hir::ExprKind::Index(ref lhs, ref rhs) => {
            let lefty = v.check_expr(lhs);
            let righty = v.check_expr(rhs);
            if v.tables.is_method_call(e) {
                return NotPromotable;
            }
            lefty & righty
        }

        hir::ExprKind::Array(ref hirvec) => {
            let mut array_result = Promotable;
            for index in hirvec.iter() {
                array_result &= v.check_expr(index);
            }
            array_result
        }

        hir::ExprKind::Type(ref expr, ref _ty) => {
            v.check_expr(&expr)
        }

        hir::ExprKind::Tup(ref hirvec) => {
            let mut tup_result = Promotable;
            for index in hirvec.iter() {
                tup_result &= v.check_expr(index);
            }
            tup_result
        }


        // Conditional control flow (possible to implement).
        hir::ExprKind::Match(ref expr, ref hirvec_arm, ref _match_source) => {
            // Compute the most demanding borrow from all the arms'
            // patterns and set that on the discriminator.
            let mut mut_borrow = false;
            for pat in hirvec_arm.iter().flat_map(|arm| &arm.pats) {
                mut_borrow = v.remove_mut_rvalue_borrow(pat);
            }
            if mut_borrow {
                v.mut_rvalue_borrows.insert(expr.id);
            }

            let _ = v.check_expr(expr);
            for index in hirvec_arm.iter() {
                let _ = v.check_expr(&*index.body);
                if let Some(hir::Guard::If(ref expr)) = index.guard {
                    let _ = v.check_expr(&expr);
                }
            }
            NotPromotable
        }

        hir::ExprKind::If(ref lhs, ref rhs, ref option_expr) => {
            let _ = v.check_expr(lhs);
            let _ = v.check_expr(rhs);
            if let Some(ref expr) = option_expr {
                let _ = v.check_expr(&expr);
            }
            NotPromotable
        }

        // Loops (not very meaningful in constants).
        hir::ExprKind::While(ref expr, ref box_block, ref _option_label) => {
            let _ = v.check_expr(expr);
            let _ = v.check_block(box_block);
            NotPromotable
        }

        hir::ExprKind::Loop(ref box_block, ref _option_label, ref _loop_source) => {
            let _ = v.check_block(box_block);
            NotPromotable
        }

        // More control flow (also not very meaningful).
        hir::ExprKind::Break(_, ref option_expr) | hir::ExprKind::Ret(ref option_expr) => {
            if let Some(ref expr) = *option_expr {
                 let _ = v.check_expr(&expr);
            }
            NotPromotable
        }

        hir::ExprKind::Continue(_) => {
            NotPromotable
        }

        // Generator expressions
        hir::ExprKind::Yield(ref expr) => {
            let _ = v.check_expr(&expr);
            NotPromotable
        }

        // Expressions with side-effects.
        hir::ExprKind::AssignOp(_, ref lhs, ref rhs) | hir::ExprKind::Assign(ref lhs, ref rhs) => {
            let _ = v.check_expr(lhs);
            let _ = v.check_expr(rhs);
            NotPromotable
        }

        hir::ExprKind::InlineAsm(ref _inline_asm, ref hirvec_lhs, ref hirvec_rhs) => {
            for index in hirvec_lhs.iter().chain(hirvec_rhs.iter()) {
                let _ = v.check_expr(index);
            }
            NotPromotable
        }
    };
    ty_result & node_result
}

/// Check the adjustments of an expression
fn check_adjustments<'a, 'tcx>(
    v: &mut CheckCrateVisitor<'a, 'tcx>,
    e: &hir::Expr) -> Promotability {
    use rustc::ty::adjustment::*;

    let mut adjustments = v.tables.expr_adjustments(e).iter().peekable();
    while let Some(adjustment) = adjustments.next() {
        match adjustment.kind {
            Adjust::NeverToAny |
            Adjust::ReifyFnPointer |
            Adjust::UnsafeFnPointer |
            Adjust::ClosureFnPointer |
            Adjust::MutToConstPointer |
            Adjust::Borrow(_) |
            Adjust::Unsize => {}

            Adjust::Deref(_) => {
                if let Some(next_adjustment) = adjustments.peek() {
                    if let Adjust::Borrow(_) = next_adjustment.kind {
                        continue;
                    }
                }
                return NotPromotable;
            }
        }
    }
    Promotable
}

impl<'a, 'gcx, 'tcx> euv::Delegate<'tcx> for CheckCrateVisitor<'a, 'gcx> {
    fn consume(&mut self,
               _consume_id: ast::NodeId,
               _consume_span: Span,
               _cmt: &mc::cmt_,
               _mode: euv::ConsumeMode) {}

    fn borrow(&mut self,
              borrow_id: ast::NodeId,
              _borrow_span: Span,
              cmt: &mc::cmt_<'tcx>,
              _loan_region: ty::Region<'tcx>,
              bk: ty::BorrowKind,
              loan_cause: euv::LoanCause) {
        debug!(
            "borrow(borrow_id={:?}, cmt={:?}, bk={:?}, loan_cause={:?})",
            borrow_id,
            cmt,
            bk,
            loan_cause,
        );

        // Kind of hacky, but we allow Unsafe coercions in constants.
        // These occur when we convert a &T or *T to a *U, as well as
        // when making a thin pointer (e.g., `*T`) into a fat pointer
        // (e.g., `*Trait`).
        if let euv::LoanCause::AutoUnsafe = loan_cause {
            return;
        }

        let mut cur = cmt;
        loop {
            match cur.cat {
                Categorization::ThreadLocal(..) |
                Categorization::Rvalue(..) => {
                    if loan_cause == euv::MatchDiscriminant {
                        // Ignore the dummy immutable borrow created by EUV.
                        break;
                    }
                    if bk.to_mutbl_lossy() == hir::MutMutable {
                        self.mut_rvalue_borrows.insert(borrow_id);
                    }
                    break;
                }
                Categorization::StaticItem => {
                    break;
                }
                Categorization::Deref(ref cmt, _) |
                Categorization::Downcast(ref cmt, _) |
                Categorization::Interior(ref cmt, _) => {
                    cur = cmt;
                }

                Categorization::Upvar(..) |
                Categorization::Local(..) => break,
            }
        }
    }

    fn decl_without_init(&mut self, _id: ast::NodeId, _span: Span) {}
    fn mutate(&mut self,
              _assignment_id: ast::NodeId,
              _assignment_span: Span,
              _assignee_cmt: &mc::cmt_,
              _mode: euv::MutateMode) {
    }

    fn matched_pat(&mut self, _: &hir::Pat, _: &mc::cmt_, _: euv::MatchMode) {}

    fn consume_pat(&mut self, _consume_pat: &hir::Pat, _cmt: &mc::cmt_, _mode: euv::ConsumeMode) {}
}
