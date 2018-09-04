// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use build;
use hair::cx::Cx;
use hair::{LintLevel, BindingMode, PatternKind};
use rustc::hir;
use rustc::hir::def_id::{DefId, LocalDefId};
use rustc::middle::region;
use rustc::mir::*;
use rustc::mir::visit::{MutVisitor, TyContext};
use rustc::ty::{self, Ty, TyCtxt};
use rustc::ty::subst::Substs;
use rustc::util::nodemap::NodeMap;
use rustc_target::spec::PanicStrategy;
use rustc_data_structures::indexed_vec::{IndexVec, Idx};
use shim;
use std::mem;
use std::u32;
use rustc_target::spec::abi::Abi;
use syntax::ast;
use syntax::attr::{self, UnwindAttr};
use syntax::symbol::keywords;
use syntax_pos::Span;
use transform::MirSource;
use util as mir_util;

/// Construct the MIR for a given def-id.
pub fn mir_build<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) -> Mir<'tcx> {
    let id = tcx.hir.as_local_node_id(def_id).unwrap();
    let unsupported = || {
        span_bug!(tcx.hir.span(id), "can't build MIR for {:?}", def_id);
    };

    // Figure out what primary body this item has.
    let body_id = match tcx.hir.get(id) {
        hir::map::NodeVariant(variant) =>
            return create_constructor_shim(tcx, id, &variant.node.data),
        hir::map::NodeStructCtor(ctor) =>
            return create_constructor_shim(tcx, id, ctor),

        _ => match tcx.hir.maybe_body_owned_by(id) {
            Some(body) => body,
            None => unsupported(),
        },
    };

    tcx.infer_ctxt().enter(|infcx| {
        let cx = Cx::new(&infcx, id);
        let mut mir = if cx.tables().tainted_by_errors {
            build::construct_error(cx, body_id)
        } else if let hir::BodyOwnerKind::Fn = cx.body_owner_kind {
            // fetch the fully liberated fn signature (that is, all bound
            // types/lifetimes replaced)
            let fn_hir_id = tcx.hir.node_to_hir_id(id);
            let fn_sig = cx.tables().liberated_fn_sigs()[fn_hir_id].clone();

            let ty = tcx.type_of(tcx.hir.local_def_id(id));
            let mut abi = fn_sig.abi;
            let implicit_argument = match ty.sty {
                ty::TyClosure(..) => {
                    // HACK(eddyb) Avoid having RustCall on closures,
                    // as it adds unnecessary (and wrong) auto-tupling.
                    abi = Abi::Rust;
                    Some(ArgInfo(liberated_closure_env_ty(tcx, id, body_id), None, None, None))
                }
                ty::TyGenerator(..) => {
                    let gen_ty = tcx.body_tables(body_id).node_id_to_type(fn_hir_id);
                    Some(ArgInfo(gen_ty, None, None, None))
                }
                _ => None,
            };

            // FIXME: safety in closures
            let safety = match fn_sig.unsafety {
                hir::Unsafety::Normal => Safety::Safe,
                hir::Unsafety::Unsafe => Safety::FnUnsafe,
            };

            let body = tcx.hir.body(body_id);
            let explicit_arguments =
                body.arguments
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| {
                        let owner_id = tcx.hir.body_owner(body_id);
                        let opt_ty_info;
                        let self_arg;
                        if let Some(ref fn_decl) = tcx.hir.fn_decl(owner_id) {
                            let ty_hir_id = fn_decl.inputs[index].hir_id;
                            let ty_span = tcx.hir.span(tcx.hir.hir_to_node_id(ty_hir_id));
                            opt_ty_info = Some(ty_span);
                            self_arg = if index == 0 && fn_decl.has_implicit_self {
                                Some(ImplicitSelfBinding)
                            } else {
                                None
                            };
                        } else {
                            opt_ty_info = None;
                            self_arg = None;
                        }
                        ArgInfo(fn_sig.inputs()[index], opt_ty_info, Some(&*arg.pat), self_arg)
                    });

            let arguments = implicit_argument.into_iter().chain(explicit_arguments);

            let (yield_ty, return_ty) = if body.is_generator {
                let gen_sig = match ty.sty {
                    ty::TyGenerator(gen_def_id, gen_substs, ..) =>
                        gen_substs.sig(gen_def_id, tcx),
                    _ =>
                        span_bug!(tcx.hir.span(id), "generator w/o generator type: {:?}", ty),
                };
                (Some(gen_sig.yield_ty), gen_sig.return_ty)
            } else {
                (None, fn_sig.output())
            };

            build::construct_fn(cx, id, arguments, safety, abi,
                                return_ty, yield_ty, body)
        } else {
            build::construct_const(cx, body_id)
        };

        // Convert the Mir to global types.
        let mut globalizer = GlobalizeMir {
            tcx,
            span: mir.span
        };
        globalizer.visit_mir(&mut mir);
        let mir = unsafe {
            mem::transmute::<Mir, Mir<'tcx>>(mir)
        };

        mir_util::dump_mir(tcx, None, "mir_map", &0,
                           MirSource::item(def_id), &mir, |_, _| Ok(()) );

        mir
    })
}

/// A pass to lift all the types and substitutions in a Mir
/// to the global tcx. Sadly, we don't have a "folder" that
/// can change 'tcx so we have to transmute afterwards.
struct GlobalizeMir<'a, 'gcx: 'a> {
    tcx: TyCtxt<'a, 'gcx, 'gcx>,
    span: Span
}

impl<'a, 'gcx: 'tcx, 'tcx> MutVisitor<'tcx> for GlobalizeMir<'a, 'gcx> {
    fn visit_ty(&mut self, ty: &mut Ty<'tcx>, _: TyContext) {
        if let Some(lifted) = self.tcx.lift(ty) {
            *ty = lifted;
        } else {
            span_bug!(self.span,
                      "found type `{:?}` with inference types/regions in MIR",
                      ty);
        }
    }

    fn visit_region(&mut self, region: &mut ty::Region<'tcx>, _: Location) {
        if let Some(lifted) = self.tcx.lift(region) {
            *region = lifted;
        } else {
            span_bug!(self.span,
                      "found region `{:?}` with inference types/regions in MIR",
                      region);
        }
    }

    fn visit_const(&mut self, constant: &mut &'tcx ty::Const<'tcx>, _: Location) {
        if let Some(lifted) = self.tcx.lift(constant) {
            *constant = lifted;
        } else {
            span_bug!(self.span,
                      "found constant `{:?}` with inference types/regions in MIR",
                      constant);
        }
    }

    fn visit_substs(&mut self, substs: &mut &'tcx Substs<'tcx>, _: Location) {
        if let Some(lifted) = self.tcx.lift(substs) {
            *substs = lifted;
        } else {
            span_bug!(self.span,
                      "found substs `{:?}` with inference types/regions in MIR",
                      substs);
        }
    }
}

fn create_constructor_shim<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                     ctor_id: ast::NodeId,
                                     v: &'tcx hir::VariantData)
                                     -> Mir<'tcx>
{
    let span = tcx.hir.span(ctor_id);
    if let hir::VariantData::Tuple(ref fields, ctor_id) = *v {
        tcx.infer_ctxt().enter(|infcx| {
            let mut mir = shim::build_adt_ctor(&infcx, ctor_id, fields, span);

            // Convert the Mir to global types.
            let tcx = infcx.tcx.global_tcx();
            let mut globalizer = GlobalizeMir {
                tcx,
                span: mir.span
            };
            globalizer.visit_mir(&mut mir);
            let mir = unsafe {
                mem::transmute::<Mir, Mir<'tcx>>(mir)
            };

            mir_util::dump_mir(tcx, None, "mir_map", &0,
                               MirSource::item(tcx.hir.local_def_id(ctor_id)),
                               &mir, |_, _| Ok(()) );

            mir
        })
    } else {
        span_bug!(span, "attempting to create MIR for non-tuple variant {:?}", v);
    }
}

///////////////////////////////////////////////////////////////////////////
// BuildMir -- walks a crate, looking for fn items and methods to build MIR from

fn liberated_closure_env_ty<'a, 'gcx, 'tcx>(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                            closure_expr_id: ast::NodeId,
                                            body_id: hir::BodyId)
                                            -> Ty<'tcx> {
    let closure_expr_hir_id = tcx.hir.node_to_hir_id(closure_expr_id);
    let closure_ty = tcx.body_tables(body_id).node_id_to_type(closure_expr_hir_id);

    let (closure_def_id, closure_substs) = match closure_ty.sty {
        ty::TyClosure(closure_def_id, closure_substs) => (closure_def_id, closure_substs),
        _ => bug!("closure expr does not have closure type: {:?}", closure_ty)
    };

    let closure_env_ty = tcx.closure_env_ty(closure_def_id, closure_substs).unwrap();
    tcx.liberate_late_bound_regions(closure_def_id, &closure_env_ty)
}

struct Builder<'a, 'gcx: 'a+'tcx, 'tcx: 'a> {
    hir: Cx<'a, 'gcx, 'tcx>,
    cfg: CFG<'tcx>,

    fn_span: Span,
    arg_count: usize,

    /// the current set of scopes, updated as we traverse;
    /// see the `scope` module for more details
    scopes: Vec<scope::Scope<'tcx>>,

    /// The current unsafe block in scope, even if it is hidden by
    /// a PushUnsafeBlock
    unpushed_unsafe: Safety,

    /// The number of `push_unsafe_block` levels in scope
    push_unsafe_count: usize,

    /// the current set of breakables; see the `scope` module for more
    /// details
    breakable_scopes: Vec<scope::BreakableScope<'tcx>>,

    /// the vector of all scopes that we have created thus far;
    /// we track this for debuginfo later
    source_scopes: IndexVec<SourceScope, SourceScopeData>,
    source_scope_local_data: IndexVec<SourceScope, SourceScopeLocalData>,
    source_scope: SourceScope,

    /// the guard-context: each time we build the guard expression for
    /// a match arm, we push onto this stack, and then pop when we
    /// finish building it.
    guard_context: Vec<GuardFrame>,

    /// Maps node ids of variable bindings to the `Local`s created for them.
    /// (A match binding can have two locals; the 2nd is for the arm's guard.)
    var_indices: NodeMap<LocalsForNode>,
    local_decls: IndexVec<Local, LocalDecl<'tcx>>,
    unit_temp: Option<Place<'tcx>>,

    /// cached block with the RESUME terminator; this is created
    /// when first set of cleanups are built.
    cached_resume_block: Option<BasicBlock>,
    /// cached block with the RETURN terminator
    cached_return_block: Option<BasicBlock>,
    /// cached block with the UNREACHABLE terminator
    cached_unreachable_block: Option<BasicBlock>,
}

impl<'a, 'gcx, 'tcx> Builder<'a, 'gcx, 'tcx> {
    fn is_bound_var_in_guard(&self, id: ast::NodeId) -> bool {
        self.guard_context.iter().any(|frame| frame.locals.iter().any(|local| local.id == id))
    }

    fn var_local_id(&self, id: ast::NodeId, for_guard: ForGuard) -> Local {
        self.var_indices[&id].local_id(for_guard)
    }
}

#[derive(Debug)]
enum LocalsForNode {
    One(Local),
    Three { val_for_guard: Local, ref_for_guard: Local, for_arm_body: Local },
}

#[derive(Debug)]
struct GuardFrameLocal {
    id: ast::NodeId,
}

impl GuardFrameLocal {
    fn new(id: ast::NodeId, _binding_mode: BindingMode) -> Self {
        GuardFrameLocal {
            id: id,
        }
    }
}

#[derive(Debug)]
struct GuardFrame {
    /// These are the id's of names that are bound by patterns of the
    /// arm of *this* guard.
    ///
    /// (Frames higher up the stack will have the id's bound in arms
    /// further out, such as in a case like:
    ///
    /// match E1 {
    ///      P1(id1) if (... (match E2 { P2(id2) if ... => B2 })) => B1,
    /// }
    ///
    /// here, when building for FIXME
    locals: Vec<GuardFrameLocal>,
}

/// ForGuard indicates whether we are talking about:
///   1. the temp for a local binding used solely within guard expressions,
///   2. the temp that holds reference to (1.), which is actually what the
///      guard expressions see, or
///   3. the temp for use outside of guard expressions.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ForGuard {
    ValWithinGuard,
    RefWithinGuard,
    OutsideGuard,
}

impl LocalsForNode {
    fn local_id(&self, for_guard: ForGuard) -> Local {
        match (self, for_guard) {
            (&LocalsForNode::One(local_id), ForGuard::OutsideGuard) |
            (&LocalsForNode::Three { val_for_guard: local_id, .. }, ForGuard::ValWithinGuard) |
            (&LocalsForNode::Three { ref_for_guard: local_id, .. }, ForGuard::RefWithinGuard) |
            (&LocalsForNode::Three { for_arm_body: local_id, .. }, ForGuard::OutsideGuard) =>
                local_id,

            (&LocalsForNode::One(_), ForGuard::ValWithinGuard) |
            (&LocalsForNode::One(_), ForGuard::RefWithinGuard) =>
                bug!("anything with one local should never be within a guard."),
        }
    }
}

struct CFG<'tcx> {
    basic_blocks: IndexVec<BasicBlock, BasicBlockData<'tcx>>,
}

newtype_index!(ScopeId);

///////////////////////////////////////////////////////////////////////////
/// The `BlockAnd` "monad" packages up the new basic block along with a
/// produced value (sometimes just unit, of course). The `unpack!`
/// macro (and methods below) makes working with `BlockAnd` much more
/// convenient.

#[must_use = "if you don't use one of these results, you're leaving a dangling edge"]
struct BlockAnd<T>(BasicBlock, T);

trait BlockAndExtension {
    fn and<T>(self, v: T) -> BlockAnd<T>;
    fn unit(self) -> BlockAnd<()>;
}

impl BlockAndExtension for BasicBlock {
    fn and<T>(self, v: T) -> BlockAnd<T> {
        BlockAnd(self, v)
    }

    fn unit(self) -> BlockAnd<()> {
        BlockAnd(self, ())
    }
}

/// Update a block pointer and return the value.
/// Use it like `let x = unpack!(block = self.foo(block, foo))`.
macro_rules! unpack {
    ($x:ident = $c:expr) => {
        {
            let BlockAnd(b, v) = $c;
            $x = b;
            v
        }
    };

    ($c:expr) => {
        {
            let BlockAnd(b, ()) = $c;
            b
        }
    };
}

fn should_abort_on_panic<'a, 'gcx, 'tcx>(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                         fn_def_id: DefId,
                                         abi: Abi)
                                         -> bool {
    // Not callable from C, so we can safely unwind through these
    if abi == Abi::Rust || abi == Abi::RustCall { return false; }

    // We never unwind, so it's not relevant to stop an unwind
    if tcx.sess.panic_strategy() != PanicStrategy::Unwind { return false; }

    // We cannot add landing pads, so don't add one
    if tcx.sess.no_landing_pads() { return false; }

    // This is a special case: some functions have a C abi but are meant to
    // unwind anyway. Don't stop them.
    let attrs = &tcx.get_attrs(fn_def_id);
    match attr::find_unwind_attr(Some(tcx.sess.diagnostic()), attrs) {
        None => {
            // FIXME(rust-lang/rust#48251) -- Had to disable
            // abort-on-panic for backwards compatibility reasons.
            false
        }

        Some(UnwindAttr::Allowed) => false,
        Some(UnwindAttr::Aborts) => true,
    }
}

///////////////////////////////////////////////////////////////////////////
/// the main entry point for building MIR for a function

struct ImplicitSelfBinding;

struct ArgInfo<'gcx>(Ty<'gcx>,
                     Option<Span>,
                     Option<&'gcx hir::Pat>,
                     Option<ImplicitSelfBinding>);

fn construct_fn<'a, 'gcx, 'tcx, A>(hir: Cx<'a, 'gcx, 'tcx>,
                                   fn_id: ast::NodeId,
                                   arguments: A,
                                   safety: Safety,
                                   abi: Abi,
                                   return_ty: Ty<'gcx>,
                                   yield_ty: Option<Ty<'gcx>>,
                                   body: &'gcx hir::Body)
                                   -> Mir<'tcx>
    where A: Iterator<Item=ArgInfo<'gcx>>
{
    let arguments: Vec<_> = arguments.collect();

    let tcx = hir.tcx();
    let span = tcx.hir.span(fn_id);
    let mut builder = Builder::new(hir.clone(),
        span,
        arguments.len(),
        safety,
        return_ty);

    let fn_def_id = tcx.hir.local_def_id(fn_id);
    let call_site_scope = region::Scope::CallSite(body.value.hir_id.local_id);
    let arg_scope = region::Scope::Arguments(body.value.hir_id.local_id);
    let mut block = START_BLOCK;
    let source_info = builder.source_info(span);
    let call_site_s = (call_site_scope, source_info);
    unpack!(block = builder.in_scope(call_site_s, LintLevel::Inherited, block, |builder| {
        if should_abort_on_panic(tcx, fn_def_id, abi) {
            builder.schedule_abort();
        }

        let arg_scope_s = (arg_scope, source_info);
        unpack!(block = builder.in_scope(arg_scope_s, LintLevel::Inherited, block, |builder| {
            builder.args_and_body(block, &arguments, arg_scope, &body.value)
        }));
        // Attribute epilogue to function's closing brace
        let fn_end = span.shrink_to_hi();
        let source_info = builder.source_info(fn_end);
        let return_block = builder.return_block();
        builder.cfg.terminate(block, source_info,
                              TerminatorKind::Goto { target: return_block });
        builder.cfg.terminate(return_block, source_info,
                              TerminatorKind::Return);
        // Attribute any unreachable codepaths to the function's closing brace
        if let Some(unreachable_block) = builder.cached_unreachable_block {
            builder.cfg.terminate(unreachable_block, source_info,
                                  TerminatorKind::Unreachable);
        }
        return_block.unit()
    }));
    assert_eq!(block, builder.return_block());

    let mut spread_arg = None;
    if abi == Abi::RustCall {
        // RustCall pseudo-ABI untuples the last argument.
        spread_arg = Some(Local::new(arguments.len()));
    }
    let closure_expr_id = tcx.hir.local_def_id(fn_id);
    info!("fn_id {:?} has attrs {:?}", closure_expr_id,
          tcx.get_attrs(closure_expr_id));

    // Gather the upvars of a closure, if any.
    let upvar_decls: Vec<_> = tcx.with_freevars(fn_id, |freevars| {
        freevars.iter().map(|fv| {
            let var_id = fv.var_id();
            let var_hir_id = tcx.hir.node_to_hir_id(var_id);
            let closure_expr_id = tcx.hir.local_def_id(fn_id);
            let capture = hir.tables().upvar_capture(ty::UpvarId {
                var_id: var_hir_id,
                closure_expr_id: LocalDefId::from_def_id(closure_expr_id),
            });
            let by_ref = match capture {
                ty::UpvarCapture::ByValue => false,
                ty::UpvarCapture::ByRef(..) => true
            };
            let mut decl = UpvarDecl {
                debug_name: keywords::Invalid.name(),
                by_ref,
                mutability: Mutability::Not,
            };
            if let Some(hir::map::NodeBinding(pat)) = tcx.hir.find(var_id) {
                if let hir::PatKind::Binding(_, _, ref name, _) = pat.node {
                    decl.debug_name = name.node;

                    let bm = *hir.tables.pat_binding_modes()
                                        .get(pat.hir_id)
                                        .expect("missing binding mode");
                    if bm == ty::BindByValue(hir::MutMutable) {
                        decl.mutability = Mutability::Mut;
                    } else {
                        decl.mutability = Mutability::Not;
                    }
                }
            }
            decl
        }).collect()
    });

    let mut mir = builder.finish(upvar_decls, yield_ty);
    mir.spread_arg = spread_arg;
    mir
}

fn construct_const<'a, 'gcx, 'tcx>(hir: Cx<'a, 'gcx, 'tcx>,
                                   body_id: hir::BodyId)
                                   -> Mir<'tcx> {
    let tcx = hir.tcx();
    let ast_expr = &tcx.hir.body(body_id).value;
    let ty = hir.tables().expr_ty_adjusted(ast_expr);
    let owner_id = tcx.hir.body_owner(body_id);
    let span = tcx.hir.span(owner_id);
    let mut builder = Builder::new(hir.clone(), span, 0, Safety::Safe, ty);

    let mut block = START_BLOCK;
    let expr = builder.hir.mirror(ast_expr);
    unpack!(block = builder.into_expr(&Place::Local(RETURN_PLACE), block, expr));

    let source_info = builder.source_info(span);
    builder.cfg.terminate(block, source_info, TerminatorKind::Return);

    // Constants can't `return` so a return block should not be created.
    assert_eq!(builder.cached_return_block, None);

    // Constants may be match expressions in which case an unreachable block may
    // be created, so terminate it properly.
    if let Some(unreachable_block) = builder.cached_unreachable_block {
        builder.cfg.terminate(unreachable_block, source_info,
                              TerminatorKind::Unreachable);
    }

    builder.finish(vec![], None)
}

fn construct_error<'a, 'gcx, 'tcx>(hir: Cx<'a, 'gcx, 'tcx>,
                                   body_id: hir::BodyId)
                                   -> Mir<'tcx> {
    let owner_id = hir.tcx().hir.body_owner(body_id);
    let span = hir.tcx().hir.span(owner_id);
    let ty = hir.tcx().types.err;
    let mut builder = Builder::new(hir, span, 0, Safety::Safe, ty);
    let source_info = builder.source_info(span);
    builder.cfg.terminate(START_BLOCK, source_info, TerminatorKind::Unreachable);
    builder.finish(vec![], None)
}

impl<'a, 'gcx, 'tcx> Builder<'a, 'gcx, 'tcx> {
    fn new(hir: Cx<'a, 'gcx, 'tcx>,
           span: Span,
           arg_count: usize,
           safety: Safety,
           return_ty: Ty<'tcx>)
           -> Builder<'a, 'gcx, 'tcx> {
        let lint_level = LintLevel::Explicit(hir.root_lint_level);
        let mut builder = Builder {
            hir,
            cfg: CFG { basic_blocks: IndexVec::new() },
            fn_span: span,
            arg_count,
            scopes: vec![],
            source_scopes: IndexVec::new(),
            source_scope: OUTERMOST_SOURCE_SCOPE,
            source_scope_local_data: IndexVec::new(),
            guard_context: vec![],
            push_unsafe_count: 0,
            unpushed_unsafe: safety,
            breakable_scopes: vec![],
            local_decls: IndexVec::from_elem_n(LocalDecl::new_return_place(return_ty,
                                                                             span), 1),
            var_indices: NodeMap(),
            unit_temp: None,
            cached_resume_block: None,
            cached_return_block: None,
            cached_unreachable_block: None,
        };

        assert_eq!(builder.cfg.start_new_block(), START_BLOCK);
        assert_eq!(
            builder.new_source_scope(span, lint_level, Some(safety)),
            OUTERMOST_SOURCE_SCOPE);
        builder.source_scopes[OUTERMOST_SOURCE_SCOPE].parent_scope = None;

        builder
    }

    fn finish(self,
              upvar_decls: Vec<UpvarDecl>,
              yield_ty: Option<Ty<'tcx>>)
              -> Mir<'tcx> {
        for (index, block) in self.cfg.basic_blocks.iter().enumerate() {
            if block.terminator.is_none() {
                span_bug!(self.fn_span, "no terminator on block {:?}", index);
            }
        }

        Mir::new(self.cfg.basic_blocks,
                 self.source_scopes,
                 ClearCrossCrate::Set(self.source_scope_local_data),
                 IndexVec::new(),
                 yield_ty,
                 self.local_decls,
                 self.arg_count,
                 upvar_decls,
                 self.fn_span
        )
    }

    fn args_and_body(&mut self,
                     mut block: BasicBlock,
                     arguments: &[ArgInfo<'gcx>],
                     argument_scope: region::Scope,
                     ast_body: &'gcx hir::Expr)
                     -> BlockAnd<()>
    {
        // Allocate locals for the function arguments
        for &ArgInfo(ty, _, pattern, _) in arguments.iter() {
            // If this is a simple binding pattern, give the local a nice name for debuginfo.
            let mut name = None;
            if let Some(pat) = pattern {
                if let hir::PatKind::Binding(_, _, ref ident, _) = pat.node {
                    name = Some(ident.node);
                }
            }

            let source_info = SourceInfo {
                scope: OUTERMOST_SOURCE_SCOPE,
                span: pattern.map_or(self.fn_span, |pat| pat.span)
            };
            self.local_decls.push(LocalDecl {
                mutability: Mutability::Mut,
                ty,
                source_info,
                visibility_scope: source_info.scope,
                name,
                internal: false,
                is_user_variable: None,
            });
        }

        let mut scope = None;
        // Bind the argument patterns
        for (index, arg_info) in arguments.iter().enumerate() {
            // Function arguments always get the first Local indices after the return place
            let local = Local::new(index + 1);
            let place = Place::Local(local);
            let &ArgInfo(ty, opt_ty_info, pattern, ref self_binding) = arg_info;

            if let Some(pattern) = pattern {
                let pattern = self.hir.pattern_from_hir(pattern);

                match *pattern.kind {
                    // Don't introduce extra copies for simple bindings
                    PatternKind::Binding { mutability, var, mode: BindingMode::ByValue, .. } => {
                        self.local_decls[local].mutability = mutability;
                        self.local_decls[local].is_user_variable =
                            if let Some(ImplicitSelfBinding) = self_binding {
                                Some(ClearCrossCrate::Set(BindingForm::ImplicitSelf))
                            } else {
                                let binding_mode = ty::BindingMode::BindByValue(mutability.into());
                                Some(ClearCrossCrate::Set(BindingForm::Var(VarBindingForm {
                                    binding_mode, opt_ty_info })))
                            };
                        self.var_indices.insert(var, LocalsForNode::One(local));
                    }
                    _ => {
                        scope = self.declare_bindings(scope, ast_body.span,
                                                      LintLevel::Inherited, &pattern,
                                                      matches::ArmHasGuard(false));
                        unpack!(block = self.place_into_pattern(block, pattern, &place));
                    }
                }
            }

            // Make sure we drop (parts of) the argument even when not matched on.
            self.schedule_drop(pattern.as_ref().map_or(ast_body.span, |pat| pat.span),
                               argument_scope, &place, ty);

        }

        // Enter the argument pattern bindings source scope, if it exists.
        if let Some(source_scope) = scope {
            self.source_scope = source_scope;
        }

        let body = self.hir.mirror(ast_body);
        self.into(&Place::Local(RETURN_PLACE), block, body)
    }

    fn get_unit_temp(&mut self) -> Place<'tcx> {
        match self.unit_temp {
            Some(ref tmp) => tmp.clone(),
            None => {
                let ty = self.hir.unit_ty();
                let fn_span = self.fn_span;
                let tmp = self.temp(ty, fn_span);
                self.unit_temp = Some(tmp.clone());
                tmp
            }
        }
    }

    fn return_block(&mut self) -> BasicBlock {
        match self.cached_return_block {
            Some(rb) => rb,
            None => {
                let rb = self.cfg.start_new_block();
                self.cached_return_block = Some(rb);
                rb
            }
        }
    }

    fn unreachable_block(&mut self) -> BasicBlock {
        match self.cached_unreachable_block {
            Some(ub) => ub,
            None => {
                let ub = self.cfg.start_new_block();
                self.cached_unreachable_block = Some(ub);
                ub
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Builder methods are broken up into modules, depending on what kind
// of thing is being lowered. Note that they use the `unpack` macro
// above extensively.

mod block;
mod cfg;
mod expr;
mod into;
mod matches;
mod misc;
mod scope;
