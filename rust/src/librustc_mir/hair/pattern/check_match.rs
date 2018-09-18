// Copyright 2012-2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::_match::{MatchCheckCtxt, Matrix, expand_pattern, is_useful};
use super::_match::Usefulness::*;
use super::_match::WitnessPreference::*;

use super::{Pattern, PatternContext, PatternError, PatternKind};

use rustc::middle::expr_use_visitor::{ConsumeMode, Delegate, ExprUseVisitor};
use rustc::middle::expr_use_visitor::{LoanCause, MutateMode};
use rustc::middle::expr_use_visitor as euv;
use rustc::middle::mem_categorization::cmt_;
use rustc::middle::region;
use rustc::session::Session;
use rustc::ty::{self, Ty, TyCtxt};
use rustc::ty::subst::Substs;
use rustc::lint;
use rustc_errors::DiagnosticBuilder;
use rustc::util::common::ErrorReported;

use rustc::hir::def::*;
use rustc::hir::def_id::DefId;
use rustc::hir::intravisit::{self, Visitor, NestedVisitorMap};
use rustc::hir::{self, Pat, PatKind};

use std::slice;

use syntax::ast;
use syntax::ptr::P;
use syntax_pos::{Span, DUMMY_SP};

struct OuterVisitor<'a, 'tcx: 'a> { tcx: TyCtxt<'a, 'tcx, 'tcx> }

impl<'a, 'tcx> Visitor<'tcx> for OuterVisitor<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::OnlyBodies(&self.tcx.hir)
    }

    fn visit_body(&mut self, body: &'tcx hir::Body) {
        intravisit::walk_body(self, body);
        let def_id = self.tcx.hir.body_owner_def_id(body.id());
        let _ = self.tcx.check_match(def_id);
    }
}

pub fn check_crate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>) {
    tcx.hir.krate().visit_all_item_likes(&mut OuterVisitor { tcx: tcx }.as_deep_visitor());
    tcx.sess.abort_if_errors();
}

pub(crate) fn check_match<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    def_id: DefId,
) -> Result<(), ErrorReported> {
    let body_id = if let Some(id) = tcx.hir.as_local_node_id(def_id) {
        tcx.hir.body_owned_by(id)
    } else {
        return Ok(());
    };

    tcx.sess.track_errors(|| {
        MatchVisitor {
            tcx,
            tables: tcx.body_tables(body_id),
            region_scope_tree: &tcx.region_scope_tree(def_id),
            param_env: tcx.param_env(def_id),
            identity_substs: Substs::identity_for_item(tcx, def_id),
        }.visit_body(tcx.hir.body(body_id));
    })
}

fn create_e0004<'a>(sess: &'a Session, sp: Span, error_message: String) -> DiagnosticBuilder<'a> {
    struct_span_err!(sess, sp, E0004, "{}", &error_message)
}

struct MatchVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    tables: &'a ty::TypeckTables<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    identity_substs: &'tcx Substs<'tcx>,
    region_scope_tree: &'a region::ScopeTree,
}

impl<'a, 'tcx> Visitor<'tcx> for MatchVisitor<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }

    fn visit_expr(&mut self, ex: &'tcx hir::Expr) {
        intravisit::walk_expr(self, ex);

        match ex.node {
            hir::ExprMatch(ref scrut, ref arms, source) => {
                self.check_match(scrut, arms, source);
            }
            _ => {}
        }
    }

    fn visit_local(&mut self, loc: &'tcx hir::Local) {
        intravisit::walk_local(self, loc);

        self.check_irrefutable(&loc.pat, match loc.source {
            hir::LocalSource::Normal => "local binding",
            hir::LocalSource::ForLoopDesugar => "`for` loop binding",
        });

        // Check legality of move bindings and `@` patterns.
        self.check_patterns(false, slice::from_ref(&loc.pat));
    }

    fn visit_body(&mut self, body: &'tcx hir::Body) {
        intravisit::walk_body(self, body);

        for arg in &body.arguments {
            self.check_irrefutable(&arg.pat, "function argument");
            self.check_patterns(false, slice::from_ref(&arg.pat));
        }
    }
}


impl<'a, 'tcx> PatternContext<'a, 'tcx> {
    fn report_inlining_errors(&self, pat_span: Span) {
        for error in &self.errors {
            match *error {
                PatternError::StaticInPattern(span) => {
                    self.span_e0158(span, "statics cannot be referenced in patterns")
                }
                PatternError::AssociatedConstInPattern(span) => {
                    self.span_e0158(span, "associated consts cannot be referenced in patterns")
                }
                PatternError::FloatBug => {
                    // FIXME(#31407) this is only necessary because float parsing is buggy
                    ::rustc::middle::const_val::struct_error(
                        self.tcx.at(pat_span),
                        "could not evaluate float literal (see issue #31407)",
                    ).emit();
                }
                PatternError::NonConstPath(span) => {
                    ::rustc::middle::const_val::struct_error(
                        self.tcx.at(span),
                        "runtime values cannot be referenced in patterns",
                    ).emit();
                }
            }
        }
    }

    fn span_e0158(&self, span: Span, text: &str) {
        span_err!(self.tcx.sess, span, E0158, "{}", text)
    }
}

impl<'a, 'tcx> MatchVisitor<'a, 'tcx> {
    fn check_patterns(&self, has_guard: bool, pats: &[P<Pat>]) {
        check_legality_of_move_bindings(self, has_guard, pats);
        for pat in pats {
            check_legality_of_bindings_in_at_patterns(self, pat);
        }
    }

    fn check_match(
        &self,
        scrut: &hir::Expr,
        arms: &'tcx [hir::Arm],
        source: hir::MatchSource)
    {
        for arm in arms {
            // First, check legality of move bindings.
            self.check_patterns(arm.guard.is_some(), &arm.pats);

            // Second, if there is a guard on each arm, make sure it isn't
            // assigning or borrowing anything mutably.
            if let Some(ref guard) = arm.guard {
                if self.tcx.check_for_mutation_in_guard_via_ast_walk() {
                    check_for_mutation_in_guard(self, &guard);
                }
            }

            // Third, perform some lints.
            for pat in &arm.pats {
                check_for_bindings_named_the_same_as_variants(self, pat);
            }
        }

        let module = self.tcx.hir.get_module_parent(scrut.id);
        MatchCheckCtxt::create_and_enter(self.tcx, module, |ref mut cx| {
            let mut have_errors = false;

            let inlined_arms : Vec<(Vec<_>, _)> = arms.iter().map(|arm| (
                arm.pats.iter().map(|pat| {
                    let mut patcx = PatternContext::new(self.tcx,
                                                        self.param_env.and(self.identity_substs),
                                                        self.tables);
                    let pattern = expand_pattern(cx, patcx.lower_pattern(&pat));
                    if !patcx.errors.is_empty() {
                        patcx.report_inlining_errors(pat.span);
                        have_errors = true;
                    }
                    (pattern, &**pat)
                }).collect(),
                arm.guard.as_ref().map(|e| &**e)
            )).collect();

            // Bail out early if inlining failed.
            if have_errors {
                return;
            }

            // Fourth, check for unreachable arms.
            check_arms(cx, &inlined_arms, source);

            // Then, if the match has no arms, check whether the scrutinee
            // is uninhabited.
            let pat_ty = self.tables.node_id_to_type(scrut.hir_id);
            let module = self.tcx.hir.get_module_parent(scrut.id);
            if inlined_arms.is_empty() {
                let scrutinee_is_uninhabited = if self.tcx.features().exhaustive_patterns {
                    self.tcx.is_ty_uninhabited_from(module, pat_ty)
                } else {
                    self.conservative_is_uninhabited(pat_ty)
                };
                if !scrutinee_is_uninhabited {
                    // We know the type is inhabited, so this must be wrong
                    let mut err = create_e0004(self.tcx.sess, scrut.span,
                                               format!("non-exhaustive patterns: type {} \
                                                        is non-empty",
                                                       pat_ty));
                    span_help!(&mut err, scrut.span,
                               "Please ensure that all possible cases are being handled; \
                                possibly adding wildcards or more match arms.");
                    err.emit();
                }
                // If the type *is* uninhabited, it's vacuously exhaustive
                return;
            }

            let matrix: Matrix = inlined_arms
                .iter()
                .filter(|&&(_, guard)| guard.is_none())
                .flat_map(|arm| &arm.0)
                .map(|pat| vec![pat.0])
                .collect();
            let scrut_ty = self.tables.node_id_to_type(scrut.hir_id);
            check_exhaustive(cx, scrut_ty, scrut.span, &matrix);
        })
    }

    fn conservative_is_uninhabited(&self, scrutinee_ty: Ty<'tcx>) -> bool {
        // "rustc-1.0-style" uncontentious uninhabitableness check
        match scrutinee_ty.sty {
            ty::TyNever => true,
            ty::TyAdt(def, _) => def.variants.is_empty(),
            _ => false
        }
    }

    fn check_irrefutable(&self, pat: &'tcx Pat, origin: &str) {
        let module = self.tcx.hir.get_module_parent(pat.id);
        MatchCheckCtxt::create_and_enter(self.tcx, module, |ref mut cx| {
            let mut patcx = PatternContext::new(self.tcx,
                                                self.param_env.and(self.identity_substs),
                                                self.tables);
            let pattern = patcx.lower_pattern(pat);
            let pattern_ty = pattern.ty;
            let pats : Matrix = vec![vec![
                expand_pattern(cx, pattern)
            ]].into_iter().collect();

            let wild_pattern = Pattern {
                ty: pattern_ty,
                span: DUMMY_SP,
                kind: box PatternKind::Wild,
            };
            let witness = match is_useful(cx, &pats, &[&wild_pattern], ConstructWitness) {
                UsefulWithWitness(witness) => witness,
                NotUseful => return,
                Useful => bug!()
            };

            let pattern_string = witness[0].single_pattern().to_string();
            let mut diag = struct_span_err!(
                self.tcx.sess, pat.span, E0005,
                "refutable pattern in {}: `{}` not covered",
                origin, pattern_string
            );
            let label_msg = match pat.node {
                PatKind::Path(hir::QPath::Resolved(None, ref path))
                        if path.segments.len() == 1 && path.segments[0].parameters.is_none() => {
                    format!("interpreted as a {} pattern, not new variable", path.def.kind_name())
                }
                _ => format!("pattern `{}` not covered", pattern_string),
            };
            diag.span_label(pat.span, label_msg);
            diag.emit();
        });
    }
}

fn check_for_bindings_named_the_same_as_variants(cx: &MatchVisitor, pat: &Pat) {
    pat.walk(|p| {
        if let PatKind::Binding(_, _, name, None) = p.node {
            let bm = *cx.tables
                        .pat_binding_modes()
                        .get(p.hir_id)
                        .expect("missing binding mode");

            if bm != ty::BindByValue(hir::MutImmutable) {
                // Nothing to check.
                return true;
            }
            let pat_ty = cx.tables.pat_ty(p);
            if let ty::TyAdt(edef, _) = pat_ty.sty {
                if edef.is_enum() && edef.variants.iter().any(|variant| {
                    variant.name == name.node && variant.ctor_kind == CtorKind::Const
                }) {
                    let ty_path = cx.tcx.item_path_str(edef.did);
                    let mut err = struct_span_warn!(cx.tcx.sess, p.span, E0170,
                        "pattern binding `{}` is named the same as one \
                         of the variants of the type `{}`",
                        name.node, ty_path);
                    help!(err,
                        "if you meant to match on a variant, \
                        consider making the path in the pattern qualified: `{}::{}`",
                        ty_path, name.node);
                    err.emit();
                }
            }
        }
        true
    });
}

/// Checks for common cases of "catchall" patterns that may not be intended as such.
fn pat_is_catchall(pat: &Pat) -> bool {
    match pat.node {
        PatKind::Binding(.., None) => true,
        PatKind::Binding(.., Some(ref s)) => pat_is_catchall(s),
        PatKind::Ref(ref s, _) => pat_is_catchall(s),
        PatKind::Tuple(ref v, _) => v.iter().all(|p| {
            pat_is_catchall(&p)
        }),
        _ => false
    }
}

// Check for unreachable patterns
fn check_arms<'a, 'tcx>(cx: &mut MatchCheckCtxt<'a, 'tcx>,
                        arms: &[(Vec<(&'a Pattern<'tcx>, &hir::Pat)>, Option<&hir::Expr>)],
                        source: hir::MatchSource)
{
    let mut seen = Matrix::empty();
    let mut catchall = None;
    let mut printed_if_let_err = false;
    for (arm_index, &(ref pats, guard)) in arms.iter().enumerate() {
        for &(pat, hir_pat) in pats {
            let v = vec![pat];

            match is_useful(cx, &seen, &v, LeaveOutWitness) {
                NotUseful => {
                    match source {
                        hir::MatchSource::IfLetDesugar { .. } => {
                            if printed_if_let_err {
                                // we already printed an irrefutable if-let pattern error.
                                // We don't want two, that's just confusing.
                            } else {
                                // find the first arm pattern so we can use its span
                                let &(ref first_arm_pats, _) = &arms[0];
                                let first_pat = &first_arm_pats[0];
                                let span = first_pat.0.span;
                                struct_span_err!(cx.tcx.sess, span, E0162,
                                                "irrefutable if-let pattern")
                                    .span_label(span, "irrefutable pattern")
                                    .emit();
                                printed_if_let_err = true;
                            }
                        },

                        hir::MatchSource::WhileLetDesugar => {
                            // find the first arm pattern so we can use its span
                            let &(ref first_arm_pats, _) = &arms[0];
                            let first_pat = &first_arm_pats[0];
                            let span = first_pat.0.span;

                            // check which arm we're on.
                            match arm_index {
                                // The arm with the user-specified pattern.
                                0 => {
                                    cx.tcx.lint_node(
                                            lint::builtin::UNREACHABLE_PATTERNS,
                                        hir_pat.id, pat.span,
                                        "unreachable pattern");
                                },
                                // The arm with the wildcard pattern.
                                1 => {
                                    struct_span_err!(cx.tcx.sess, span, E0165,
                                                     "irrefutable while-let pattern")
                                        .span_label(span, "irrefutable pattern")
                                        .emit();
                                },
                                _ => bug!(),
                            }
                        },

                        hir::MatchSource::ForLoopDesugar |
                        hir::MatchSource::Normal => {
                            let mut err = cx.tcx.struct_span_lint_node(
                                lint::builtin::UNREACHABLE_PATTERNS,
                                hir_pat.id,
                                pat.span,
                                "unreachable pattern",
                            );
                            // if we had a catchall pattern, hint at that
                            if let Some(catchall) = catchall {
                                err.span_label(pat.span, "unreachable pattern");
                                err.span_label(catchall, "matches any value");
                            }
                            err.emit();
                        },

                        // Unreachable patterns in try expressions occur when one of the arms
                        // are an uninhabited type. Which is OK.
                        hir::MatchSource::TryDesugar => {}
                    }
                }
                Useful => (),
                UsefulWithWitness(_) => bug!()
            }
            if guard.is_none() {
                seen.push(v);
                if catchall.is_none() && pat_is_catchall(hir_pat) {
                    catchall = Some(pat.span);
                }
            }
        }
    }
}

fn check_exhaustive<'a, 'tcx>(cx: &mut MatchCheckCtxt<'a, 'tcx>,
                              scrut_ty: Ty<'tcx>,
                              sp: Span,
                              matrix: &Matrix<'a, 'tcx>) {
    let wild_pattern = Pattern {
        ty: scrut_ty,
        span: DUMMY_SP,
        kind: box PatternKind::Wild,
    };
    match is_useful(cx, matrix, &[&wild_pattern], ConstructWitness) {
        UsefulWithWitness(pats) => {
            let witnesses = if pats.is_empty() {
                vec![&wild_pattern]
            } else {
                pats.iter().map(|w| w.single_pattern()).collect()
            };

            const LIMIT: usize = 3;
            let joined_patterns = match witnesses.len() {
                0 => bug!(),
                1 => format!("`{}`", witnesses[0]),
                2...LIMIT => {
                    let (tail, head) = witnesses.split_last().unwrap();
                    let head: Vec<_> = head.iter().map(|w| w.to_string()).collect();
                    format!("`{}` and `{}`", head.join("`, `"), tail)
                },
                _ => {
                    let (head, tail) = witnesses.split_at(LIMIT);
                    let head: Vec<_> = head.iter().map(|w| w.to_string()).collect();
                    format!("`{}` and {} more", head.join("`, `"), tail.len())
                }
            };

            let label_text = match witnesses.len() {
                1 => format!("pattern {} not covered", joined_patterns),
                _ => format!("patterns {} not covered", joined_patterns)
            };
            create_e0004(cx.tcx.sess, sp,
                            format!("non-exhaustive patterns: {} not covered",
                                    joined_patterns))
                .span_label(sp, label_text)
                .emit();
        }
        NotUseful => {
            // This is good, wildcard pattern isn't reachable
        },
        _ => bug!()
    }
}

// Legality of move bindings checking
fn check_legality_of_move_bindings(cx: &MatchVisitor,
                                   has_guard: bool,
                                   pats: &[P<Pat>]) {
    let mut by_ref_span = None;
    for pat in pats {
        pat.each_binding(|_, hir_id, span, _path| {
            let bm = *cx.tables
                        .pat_binding_modes()
                        .get(hir_id)
                        .expect("missing binding mode");
            if let ty::BindByReference(..) = bm {
                by_ref_span = Some(span);
            }
        })
    }

    let check_move = |p: &Pat, sub: Option<&Pat>| {
        // check legality of moving out of the enum

        // x @ Foo(..) is legal, but x @ Foo(y) isn't.
        if sub.map_or(false, |p| p.contains_bindings()) {
            struct_span_err!(cx.tcx.sess, p.span, E0007,
                             "cannot bind by-move with sub-bindings")
                .span_label(p.span, "binds an already bound by-move value by moving it")
                .emit();
        } else if has_guard {
            struct_span_err!(cx.tcx.sess, p.span, E0008,
                      "cannot bind by-move into a pattern guard")
                .span_label(p.span, "moves value into pattern guard")
                .emit();
        } else if by_ref_span.is_some() {
            struct_span_err!(cx.tcx.sess, p.span, E0009,
                            "cannot bind by-move and by-ref in the same pattern")
                    .span_label(p.span, "by-move pattern here")
                    .span_label(by_ref_span.unwrap(), "both by-ref and by-move used")
                    .emit();
        }
    };

    for pat in pats {
        pat.walk(|p| {
            if let PatKind::Binding(_, _, _, ref sub) = p.node {
                let bm = *cx.tables
                            .pat_binding_modes()
                            .get(p.hir_id)
                            .expect("missing binding mode");
                match bm {
                    ty::BindByValue(..) => {
                        let pat_ty = cx.tables.node_id_to_type(p.hir_id);
                        if pat_ty.moves_by_default(cx.tcx, cx.param_env, pat.span) {
                            check_move(p, sub.as_ref().map(|p| &**p));
                        }
                    }
                    _ => {}
                }
            }
            true
        });
    }
}

/// Ensures that a pattern guard doesn't borrow by mutable reference or
/// assign.
///
/// FIXME: this should be done by borrowck.
fn check_for_mutation_in_guard(cx: &MatchVisitor, guard: &hir::Expr) {
    let mut checker = MutationChecker {
        cx,
    };
    ExprUseVisitor::new(&mut checker, cx.tcx, cx.param_env, cx.region_scope_tree, cx.tables, None)
        .walk_expr(guard);
}

struct MutationChecker<'a, 'tcx: 'a> {
    cx: &'a MatchVisitor<'a, 'tcx>,
}

impl<'a, 'tcx> Delegate<'tcx> for MutationChecker<'a, 'tcx> {
    fn matched_pat(&mut self, _: &Pat, _: &cmt_, _: euv::MatchMode) {}
    fn consume(&mut self, _: ast::NodeId, _: Span, _: &cmt_, _: ConsumeMode) {}
    fn consume_pat(&mut self, _: &Pat, _: &cmt_, _: ConsumeMode) {}
    fn borrow(&mut self,
              _: ast::NodeId,
              span: Span,
              _: &cmt_,
              _: ty::Region<'tcx>,
              kind:ty:: BorrowKind,
              _: LoanCause) {
        match kind {
            ty::MutBorrow => {
                struct_span_err!(self.cx.tcx.sess, span, E0301,
                          "cannot mutably borrow in a pattern guard")
                    .span_label(span, "borrowed mutably in pattern guard")
                    .emit();
            }
            ty::ImmBorrow | ty::UniqueImmBorrow => {}
        }
    }
    fn decl_without_init(&mut self, _: ast::NodeId, _: Span) {}
    fn mutate(&mut self, _: ast::NodeId, span: Span, _: &cmt_, mode: MutateMode) {
        match mode {
            MutateMode::JustWrite | MutateMode::WriteAndRead => {
                struct_span_err!(self.cx.tcx.sess, span, E0302, "cannot assign in a pattern guard")
                    .span_label(span, "assignment in pattern guard")
                    .emit();
            }
            MutateMode::Init => {}
        }
    }
}

/// Forbids bindings in `@` patterns. This is necessary for memory safety,
/// because of the way rvalues are handled in the borrow check. (See issue
/// #14587.)
fn check_legality_of_bindings_in_at_patterns(cx: &MatchVisitor, pat: &Pat) {
    AtBindingPatternVisitor { cx: cx, bindings_allowed: true }.visit_pat(pat);
}

struct AtBindingPatternVisitor<'a, 'b:'a, 'tcx:'b> {
    cx: &'a MatchVisitor<'b, 'tcx>,
    bindings_allowed: bool
}

impl<'a, 'b, 'tcx, 'v> Visitor<'v> for AtBindingPatternVisitor<'a, 'b, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'v> {
        NestedVisitorMap::None
    }

    fn visit_pat(&mut self, pat: &Pat) {
        match pat.node {
            PatKind::Binding(.., ref subpat) => {
                if !self.bindings_allowed {
                    struct_span_err!(self.cx.tcx.sess, pat.span, E0303,
                                     "pattern bindings are not allowed after an `@`")
                        .span_label(pat.span,  "not allowed after `@`")
                        .emit();
                }

                if subpat.is_some() {
                    let bindings_were_allowed = self.bindings_allowed;
                    self.bindings_allowed = false;
                    intravisit::walk_pat(self, pat);
                    self.bindings_allowed = bindings_were_allowed;
                }
            }
            _ => intravisit::walk_pat(self, pat),
        }
    }
}
