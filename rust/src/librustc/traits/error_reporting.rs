// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::{
    FulfillmentError,
    FulfillmentErrorCode,
    MismatchedProjectionTypes,
    Obligation,
    ObligationCause,
    ObligationCauseCode,
    OnUnimplementedDirective,
    OnUnimplementedNote,
    OutputTypeParameterMismatch,
    TraitNotObjectSafe,
    ConstEvalFailure,
    PredicateObligation,
    SelectionContext,
    SelectionError,
    ObjectSafetyViolation,
    Overflow,
};

use errors::{Applicability, DiagnosticBuilder};
use hir;
use hir::def_id::DefId;
use infer::{self, InferCtxt};
use infer::type_variable::TypeVariableOrigin;
use std::fmt;
use syntax::ast;
use session::DiagnosticMessageId;
use ty::{self, AdtKind, ToPredicate, ToPolyTraitRef, Ty, TyCtxt, TypeFoldable};
use ty::GenericParamDefKind;
use ty::error::ExpectedFound;
use ty::fast_reject;
use ty::fold::TypeFolder;
use ty::subst::Subst;
use ty::SubtypePredicate;
use util::nodemap::{FxHashMap, FxHashSet};

use syntax_pos::{DUMMY_SP, Span};

impl<'a, 'gcx, 'tcx> InferCtxt<'a, 'gcx, 'tcx> {
    pub fn report_fulfillment_errors(&self,
                                     errors: &Vec<FulfillmentError<'tcx>>,
                                     body_id: Option<hir::BodyId>,
                                     fallback_has_occurred: bool) {
        #[derive(Debug)]
        struct ErrorDescriptor<'tcx> {
            predicate: ty::Predicate<'tcx>,
            index: Option<usize>, // None if this is an old error
        }

        let mut error_map : FxHashMap<_, _> =
            self.reported_trait_errors.borrow().iter().map(|(&span, predicates)| {
                (span, predicates.iter().map(|predicate| ErrorDescriptor {
                    predicate: predicate.clone(),
                    index: None
                }).collect())
            }).collect();

        for (index, error) in errors.iter().enumerate() {
            error_map.entry(error.obligation.cause.span).or_insert(Vec::new()).push(
                ErrorDescriptor {
                    predicate: error.obligation.predicate.clone(),
                    index: Some(index)
                });

            self.reported_trait_errors.borrow_mut()
                .entry(error.obligation.cause.span).or_insert(Vec::new())
                .push(error.obligation.predicate.clone());
        }

        // We do this in 2 passes because we want to display errors in order, tho
        // maybe it *is* better to sort errors by span or something.
        let mut is_suppressed: Vec<bool> = errors.iter().map(|_| false).collect();
        for (_, error_set) in error_map.iter() {
            // We want to suppress "duplicate" errors with the same span.
            for error in error_set {
                if let Some(index) = error.index {
                    // Suppress errors that are either:
                    // 1) strictly implied by another error.
                    // 2) implied by an error with a smaller index.
                    for error2 in error_set {
                        if error2.index.map_or(false, |index2| is_suppressed[index2]) {
                            // Avoid errors being suppressed by already-suppressed
                            // errors, to prevent all errors from being suppressed
                            // at once.
                            continue
                        }

                        if self.error_implies(&error2.predicate, &error.predicate) &&
                            !(error2.index >= error.index &&
                              self.error_implies(&error.predicate, &error2.predicate))
                        {
                            info!("skipping {:?} (implied by {:?})", error, error2);
                            is_suppressed[index] = true;
                            break
                        }
                    }
                }
            }
        }

        for (error, suppressed) in errors.iter().zip(is_suppressed) {
            if !suppressed {
                self.report_fulfillment_error(error, body_id, fallback_has_occurred);
            }
        }
    }

    // returns if `cond` not occurring implies that `error` does not occur - i.e. that
    // `error` occurring implies that `cond` occurs.
    fn error_implies(&self,
                     cond: &ty::Predicate<'tcx>,
                     error: &ty::Predicate<'tcx>)
                     -> bool
    {
        if cond == error {
            return true
        }

        let (cond, error) = match (cond, error) {
            (&ty::Predicate::Trait(..), &ty::Predicate::Trait(ref error))
                => (cond, error),
            _ => {
                // FIXME: make this work in other cases too.
                return false
            }
        };

        for implication in super::elaborate_predicates(self.tcx, vec![cond.clone()]) {
            if let ty::Predicate::Trait(implication) = implication {
                let error = error.to_poly_trait_ref();
                let implication = implication.to_poly_trait_ref();
                // FIXME: I'm just not taking associated types at all here.
                // Eventually I'll need to implement param-env-aware
                // `Γ₁ ⊦ φ₁ => Γ₂ ⊦ φ₂` logic.
                let param_env = ty::ParamEnv::empty();
                if let Ok(_) = self.can_sub(param_env, error, implication) {
                    debug!("error_implies: {:?} -> {:?} -> {:?}", cond, error, implication);
                    return true
                }
            }
        }

        false
    }

    fn report_fulfillment_error(&self, error: &FulfillmentError<'tcx>,
                                body_id: Option<hir::BodyId>,
                                fallback_has_occurred: bool) {
        debug!("report_fulfillment_errors({:?})", error);
        match error.code {
            FulfillmentErrorCode::CodeSelectionError(ref e) => {
                self.report_selection_error(&error.obligation, e, fallback_has_occurred);
            }
            FulfillmentErrorCode::CodeProjectionError(ref e) => {
                self.report_projection_error(&error.obligation, e);
            }
            FulfillmentErrorCode::CodeAmbiguity => {
                self.maybe_report_ambiguity(&error.obligation, body_id);
            }
            FulfillmentErrorCode::CodeSubtypeError(ref expected_found, ref err) => {
                self.report_mismatched_types(&error.obligation.cause,
                                             expected_found.expected,
                                             expected_found.found,
                                             err.clone())
                    .emit();
            }
        }
    }

    fn report_projection_error(&self,
                               obligation: &PredicateObligation<'tcx>,
                               error: &MismatchedProjectionTypes<'tcx>)
    {
        let predicate =
            self.resolve_type_vars_if_possible(&obligation.predicate);

        if predicate.references_error() {
            return
        }

        self.probe(|_| {
            let err_buf;
            let mut err = &error.err;
            let mut values = None;

            // try to find the mismatched types to report the error with.
            //
            // this can fail if the problem was higher-ranked, in which
            // cause I have no idea for a good error message.
            if let ty::Predicate::Projection(ref data) = predicate {
                let mut selcx = SelectionContext::new(self);
                let (data, _) = self.replace_late_bound_regions_with_fresh_var(
                    obligation.cause.span,
                    infer::LateBoundRegionConversionTime::HigherRankedType,
                    data);
                let mut obligations = vec![];
                let normalized_ty = super::normalize_projection_type(
                    &mut selcx,
                    obligation.param_env,
                    data.projection_ty,
                    obligation.cause.clone(),
                    0,
                    &mut obligations
                );
                if let Err(error) = self.at(&obligation.cause, obligation.param_env)
                                        .eq(normalized_ty, data.ty) {
                    values = Some(infer::ValuePairs::Types(ExpectedFound {
                        expected: normalized_ty,
                        found: data.ty,
                    }));
                    err_buf = error;
                    err = &err_buf;
                }
            }

            let msg = format!("type mismatch resolving `{}`", predicate);
            let error_id = (DiagnosticMessageId::ErrorId(271),
                            Some(obligation.cause.span), msg.clone());
            let fresh = self.tcx.sess.one_time_diagnostics.borrow_mut().insert(error_id);
            if fresh {
                let mut diag = struct_span_err!(
                    self.tcx.sess, obligation.cause.span, E0271,
                    "type mismatch resolving `{}`", predicate
                );
                self.note_type_err(&mut diag, &obligation.cause, None, values, err);
                self.note_obligation_cause(&mut diag, obligation);
                diag.emit();
            }
        });
    }

    fn fuzzy_match_tys(&self, a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
        /// returns the fuzzy category of a given type, or None
        /// if the type can be equated to any type.
        fn type_category<'tcx>(t: Ty<'tcx>) -> Option<u32> {
            match t.sty {
                ty::TyBool => Some(0),
                ty::TyChar => Some(1),
                ty::TyStr => Some(2),
                ty::TyInt(..) | ty::TyUint(..) | ty::TyInfer(ty::IntVar(..)) => Some(3),
                ty::TyFloat(..) | ty::TyInfer(ty::FloatVar(..)) => Some(4),
                ty::TyRef(..) | ty::TyRawPtr(..) => Some(5),
                ty::TyArray(..) | ty::TySlice(..) => Some(6),
                ty::TyFnDef(..) | ty::TyFnPtr(..) => Some(7),
                ty::TyDynamic(..) => Some(8),
                ty::TyClosure(..) => Some(9),
                ty::TyTuple(..) => Some(10),
                ty::TyProjection(..) => Some(11),
                ty::TyParam(..) => Some(12),
                ty::TyAnon(..) => Some(13),
                ty::TyNever => Some(14),
                ty::TyAdt(adt, ..) => match adt.adt_kind() {
                    AdtKind::Struct => Some(15),
                    AdtKind::Union => Some(16),
                    AdtKind::Enum => Some(17),
                },
                ty::TyGenerator(..) => Some(18),
                ty::TyForeign(..) => Some(19),
                ty::TyGeneratorWitness(..) => Some(20),
                ty::TyInfer(..) | ty::TyError => None
            }
        }

        match (type_category(a), type_category(b)) {
            (Some(cat_a), Some(cat_b)) => match (&a.sty, &b.sty) {
                (&ty::TyAdt(def_a, _), &ty::TyAdt(def_b, _)) => def_a == def_b,
                _ => cat_a == cat_b
            },
            // infer and error can be equated to all types
            _ => true
        }
    }

    fn impl_similar_to(&self,
                       trait_ref: ty::PolyTraitRef<'tcx>,
                       obligation: &PredicateObligation<'tcx>)
                       -> Option<DefId>
    {
        let tcx = self.tcx;
        let param_env = obligation.param_env;
        let trait_ref = tcx.erase_late_bound_regions(&trait_ref);
        let trait_self_ty = trait_ref.self_ty();

        let mut self_match_impls = vec![];
        let mut fuzzy_match_impls = vec![];

        self.tcx.for_each_relevant_impl(
            trait_ref.def_id, trait_self_ty, |def_id| {
                let impl_substs = self.fresh_substs_for_item(obligation.cause.span, def_id);
                let impl_trait_ref = tcx
                    .impl_trait_ref(def_id)
                    .unwrap()
                    .subst(tcx, impl_substs);

                let impl_self_ty = impl_trait_ref.self_ty();

                if let Ok(..) = self.can_eq(param_env, trait_self_ty, impl_self_ty) {
                    self_match_impls.push(def_id);

                    if trait_ref.substs.types().skip(1)
                        .zip(impl_trait_ref.substs.types().skip(1))
                        .all(|(u,v)| self.fuzzy_match_tys(u, v))
                    {
                        fuzzy_match_impls.push(def_id);
                    }
                }
            });

        let impl_def_id = if self_match_impls.len() == 1 {
            self_match_impls[0]
        } else if fuzzy_match_impls.len() == 1 {
            fuzzy_match_impls[0]
        } else {
            return None
        };

        if tcx.has_attr(impl_def_id, "rustc_on_unimplemented") {
            Some(impl_def_id)
        } else {
            None
        }
    }

    fn on_unimplemented_note(
        &self,
        trait_ref: ty::PolyTraitRef<'tcx>,
        obligation: &PredicateObligation<'tcx>) ->
        OnUnimplementedNote
    {
        let def_id = self.impl_similar_to(trait_ref, obligation)
            .unwrap_or(trait_ref.def_id());
        let trait_ref = *trait_ref.skip_binder();

        let mut flags = vec![];
        match obligation.cause.code {
            ObligationCauseCode::BuiltinDerivedObligation(..) |
            ObligationCauseCode::ImplDerivedObligation(..) => {}
            _ => {
                // this is a "direct", user-specified, rather than derived,
                // obligation.
                flags.push(("direct".to_string(), None));
            }
        }

        if let ObligationCauseCode::ItemObligation(item) = obligation.cause.code {
            // FIXME: maybe also have some way of handling methods
            // from other traits? That would require name resolution,
            // which we might want to be some sort of hygienic.
            //
            // Currently I'm leaving it for what I need for `try`.
            if self.tcx.trait_of_item(item) == Some(trait_ref.def_id) {
                let method = self.tcx.item_name(item);
                flags.push(("from_method".to_string(), None));
                flags.push(("from_method".to_string(), Some(method.to_string())));
            }
        }

        if let Some(k) = obligation.cause.span.compiler_desugaring_kind() {
            let desugaring = k.as_symbol().as_str();
            flags.push(("from_desugaring".to_string(), None));
            flags.push(("from_desugaring".to_string(), Some(desugaring.to_string())));
        }
        let generics = self.tcx.generics_of(def_id);
        let self_ty = trait_ref.self_ty();
        // This is also included through the generics list as `Self`,
        // but the parser won't allow you to use it
        flags.push(("_Self".to_string(), Some(self_ty.to_string())));
        if let Some(def) = self_ty.ty_adt_def() {
            // We also want to be able to select self's original
            // signature with no type arguments resolved
            flags.push(("_Self".to_string(), Some(self.tcx.type_of(def.did).to_string())));
        }

        for param in generics.params.iter() {
            let value = match param.kind {
                GenericParamDefKind::Type {..} => {
                    trait_ref.substs[param.index as usize].to_string()
                },
                GenericParamDefKind::Lifetime => continue,
            };
            let name = param.name.to_string();
            flags.push((name, Some(value)));
        }

        if let Some(true) = self_ty.ty_to_def_id().map(|def_id| def_id.is_local()) {
            flags.push(("crate_local".to_string(), None));
        }

        if let Ok(Some(command)) = OnUnimplementedDirective::of_item(
            self.tcx, trait_ref.def_id, def_id
        ) {
            command.evaluate(self.tcx, trait_ref, &flags[..])
        } else {
            OnUnimplementedNote::empty()
        }
    }

    fn find_similar_impl_candidates(&self,
                                    trait_ref: ty::PolyTraitRef<'tcx>)
                                    -> Vec<ty::TraitRef<'tcx>>
    {
        let simp = fast_reject::simplify_type(self.tcx,
                                              trait_ref.skip_binder().self_ty(),
                                              true);
        let mut impl_candidates = Vec::new();

        match simp {
            Some(simp) => self.tcx.for_each_impl(trait_ref.def_id(), |def_id| {
                let imp = self.tcx.impl_trait_ref(def_id).unwrap();
                let imp_simp = fast_reject::simplify_type(self.tcx,
                                                          imp.self_ty(),
                                                          true);
                if let Some(imp_simp) = imp_simp {
                    if simp != imp_simp {
                        return;
                    }
                }
                impl_candidates.push(imp);
            }),
            None => self.tcx.for_each_impl(trait_ref.def_id(), |def_id| {
                impl_candidates.push(
                    self.tcx.impl_trait_ref(def_id).unwrap());
            })
        };
        impl_candidates
    }

    fn report_similar_impl_candidates(&self,
                                      impl_candidates: Vec<ty::TraitRef<'tcx>>,
                                      err: &mut DiagnosticBuilder)
    {
        if impl_candidates.is_empty() {
            return;
        }

        let end = if impl_candidates.len() <= 5 {
            impl_candidates.len()
        } else {
            4
        };

        let normalize = |candidate| self.tcx.global_tcx().infer_ctxt().enter(|ref infcx| {
            let normalized = infcx
                .at(&ObligationCause::dummy(), ty::ParamEnv::empty())
                .normalize(candidate)
                .ok();
            match normalized {
                Some(normalized) => format!("\n  {:?}", normalized.value),
                None => format!("\n  {:?}", candidate),
            }
        });

        err.help(&format!("the following implementations were found:{}{}",
                          &impl_candidates[0..end].iter().map(normalize).collect::<String>(),
                          if impl_candidates.len() > 5 {
                              format!("\nand {} others", impl_candidates.len() - 4)
                          } else {
                              "".to_owned()
                          }
                          ));
    }

    /// Reports that an overflow has occurred and halts compilation. We
    /// halt compilation unconditionally because it is important that
    /// overflows never be masked -- they basically represent computations
    /// whose result could not be truly determined and thus we can't say
    /// if the program type checks or not -- and they are unusual
    /// occurrences in any case.
    pub fn report_overflow_error<T>(&self,
                                    obligation: &Obligation<'tcx, T>,
                                    suggest_increasing_limit: bool) -> !
        where T: fmt::Display + TypeFoldable<'tcx>
    {
        let predicate =
            self.resolve_type_vars_if_possible(&obligation.predicate);
        let mut err = struct_span_err!(self.tcx.sess, obligation.cause.span, E0275,
                                       "overflow evaluating the requirement `{}`",
                                       predicate);

        if suggest_increasing_limit {
            self.suggest_new_overflow_limit(&mut err);
        }

        self.note_obligation_cause(&mut err, obligation);

        err.emit();
        self.tcx.sess.abort_if_errors();
        bug!();
    }

    /// Reports that a cycle was detected which led to overflow and halts
    /// compilation. This is equivalent to `report_overflow_error` except
    /// that we can give a more helpful error message (and, in particular,
    /// we do not suggest increasing the overflow limit, which is not
    /// going to help).
    pub fn report_overflow_error_cycle(&self, cycle: &[PredicateObligation<'tcx>]) -> ! {
        let cycle = self.resolve_type_vars_if_possible(&cycle.to_owned());
        assert!(cycle.len() > 0);

        debug!("report_overflow_error_cycle: cycle={:?}", cycle);

        self.report_overflow_error(&cycle[0], false);
    }

    pub fn report_extra_impl_obligation(&self,
                                        error_span: Span,
                                        item_name: ast::Name,
                                        _impl_item_def_id: DefId,
                                        trait_item_def_id: DefId,
                                        requirement: &dyn fmt::Display)
                                        -> DiagnosticBuilder<'tcx>
    {
        let msg = "impl has stricter requirements than trait";
        let sp = self.tcx.sess.codemap().def_span(error_span);

        let mut err = struct_span_err!(self.tcx.sess, sp, E0276, "{}", msg);

        if let Some(trait_item_span) = self.tcx.hir.span_if_local(trait_item_def_id) {
            let span = self.tcx.sess.codemap().def_span(trait_item_span);
            err.span_label(span, format!("definition of `{}` from trait", item_name));
        }

        err.span_label(sp, format!("impl has extra requirement {}", requirement));

        err
    }


    /// Get the parent trait chain start
    fn get_parent_trait_ref(&self, code: &ObligationCauseCode<'tcx>) -> Option<String> {
        match code {
            &ObligationCauseCode::BuiltinDerivedObligation(ref data) => {
                let parent_trait_ref = self.resolve_type_vars_if_possible(
                    &data.parent_trait_ref);
                match self.get_parent_trait_ref(&data.parent_code) {
                    Some(t) => Some(t),
                    None => Some(format!("{}", parent_trait_ref.skip_binder().self_ty())),
                }
            }
            _ => None,
        }
    }

    pub fn report_selection_error(&self,
                                  obligation: &PredicateObligation<'tcx>,
                                  error: &SelectionError<'tcx>,
                                  fallback_has_occurred: bool)
    {
        let span = obligation.cause.span;

        let mut err = match *error {
            SelectionError::Unimplemented => {
                if let ObligationCauseCode::CompareImplMethodObligation {
                    item_name, impl_item_def_id, trait_item_def_id,
                } = obligation.cause.code {
                    self.report_extra_impl_obligation(
                        span,
                        item_name,
                        impl_item_def_id,
                        trait_item_def_id,
                        &format!("`{}`", obligation.predicate))
                        .emit();
                    return;
                }
                match obligation.predicate {
                    ty::Predicate::Trait(ref trait_predicate) => {
                        let trait_predicate =
                            self.resolve_type_vars_if_possible(trait_predicate);

                        if self.tcx.sess.has_errors() && trait_predicate.references_error() {
                            return;
                        }
                        let trait_ref = trait_predicate.to_poly_trait_ref();
                        let (post_message, pre_message) =
                            self.get_parent_trait_ref(&obligation.cause.code)
                                .map(|t| (format!(" in `{}`", t), format!("within `{}`, ", t)))
                            .unwrap_or((String::new(), String::new()));

                        let OnUnimplementedNote { message, label, note }
                            = self.on_unimplemented_note(trait_ref, obligation);
                        let have_alt_message = message.is_some() || label.is_some();

                        let mut err = struct_span_err!(
                            self.tcx.sess,
                            span,
                            E0277,
                            "{}",
                            message.unwrap_or_else(|| {
                                format!("the trait bound `{}` is not satisfied{}",
                                         trait_ref.to_predicate(), post_message)
                            }));

                        let explanation =
                            if obligation.cause.code == ObligationCauseCode::MainFunctionType {
                                "consider using `()`, or a `Result`".to_owned()
                            } else {
                                format!("{}the trait `{}` is not implemented for `{}`",
                                        pre_message,
                                        trait_ref,
                                        trait_ref.self_ty())
                            };

                        if let Some(ref s) = label {
                            // If it has a custom "#[rustc_on_unimplemented]"
                            // error message, let's display it as the label!
                            err.span_label(span, s.as_str());
                            err.help(&explanation);
                        } else {
                            err.span_label(span, explanation);
                        }
                        if let Some(ref s) = note {
                            // If it has a custom "#[rustc_on_unimplemented]" note, let's display it
                            err.note(s.as_str());
                        }

                        self.suggest_borrow_on_unsized_slice(&obligation.cause.code, &mut err);
                        self.suggest_remove_reference(&obligation, &mut err, &trait_ref);

                        // Try to report a help message
                        if !trait_ref.has_infer_types() &&
                            self.predicate_can_apply(obligation.param_env, trait_ref) {
                            // If a where-clause may be useful, remind the
                            // user that they can add it.
                            //
                            // don't display an on-unimplemented note, as
                            // these notes will often be of the form
                            //     "the type `T` can't be frobnicated"
                            // which is somewhat confusing.
                            err.help(&format!("consider adding a `where {}` bound",
                                                trait_ref.to_predicate()));
                        } else if !have_alt_message {
                            // Can't show anything else useful, try to find similar impls.
                            let impl_candidates = self.find_similar_impl_candidates(trait_ref);
                            self.report_similar_impl_candidates(impl_candidates, &mut err);
                        }

                        // If this error is due to `!: Trait` not implemented but `(): Trait` is
                        // implemented, and fallback has occured, then it could be due to a
                        // variable that used to fallback to `()` now falling back to `!`. Issue a
                        // note informing about the change in behaviour.
                        if trait_predicate.skip_binder().self_ty().is_never()
                            && fallback_has_occurred
                        {
                            let predicate = trait_predicate.map_bound(|mut trait_pred| {
                                trait_pred.trait_ref.substs = self.tcx.mk_substs_trait(
                                    self.tcx.mk_nil(),
                                    &trait_pred.trait_ref.substs[1..],
                                );
                                trait_pred
                            });
                            let unit_obligation = Obligation {
                                predicate: ty::Predicate::Trait(predicate),
                                .. obligation.clone()
                            };
                            if self.predicate_may_hold(&unit_obligation) {
                                err.note("the trait is implemented for `()`. \
                                         Possibly this error has been caused by changes to \
                                         Rust's type-inference algorithm \
                                         (see: https://github.com/rust-lang/rust/issues/48950 \
                                         for more info). Consider whether you meant to use the \
                                         type `()` here instead.");
                            }
                        }

                        err
                    }

                    ty::Predicate::Subtype(ref predicate) => {
                        // Errors for Subtype predicates show up as
                        // `FulfillmentErrorCode::CodeSubtypeError`,
                        // not selection error.
                        span_bug!(span, "subtype requirement gave wrong error: `{:?}`", predicate)
                    }

                    ty::Predicate::RegionOutlives(ref predicate) => {
                        let predicate = self.resolve_type_vars_if_possible(predicate);
                        let err = self.region_outlives_predicate(&obligation.cause,
                                                                    &predicate).err().unwrap();
                        struct_span_err!(self.tcx.sess, span, E0279,
                            "the requirement `{}` is not satisfied (`{}`)",
                            predicate, err)
                    }

                    ty::Predicate::Projection(..) | ty::Predicate::TypeOutlives(..) => {
                        let predicate =
                            self.resolve_type_vars_if_possible(&obligation.predicate);
                        struct_span_err!(self.tcx.sess, span, E0280,
                            "the requirement `{}` is not satisfied",
                            predicate)
                    }

                    ty::Predicate::ObjectSafe(trait_def_id) => {
                        let violations = self.tcx.object_safety_violations(trait_def_id);
                        self.tcx.report_object_safety_error(span,
                                                            trait_def_id,
                                                            violations)
                    }

                    ty::Predicate::ClosureKind(closure_def_id, closure_substs, kind) => {
                        let found_kind = self.closure_kind(closure_def_id, closure_substs).unwrap();
                        let closure_span = self.tcx.sess.codemap()
                            .def_span(self.tcx.hir.span_if_local(closure_def_id).unwrap());
                        let node_id = self.tcx.hir.as_local_node_id(closure_def_id).unwrap();
                        let mut err = struct_span_err!(
                            self.tcx.sess, closure_span, E0525,
                            "expected a closure that implements the `{}` trait, \
                                but this closure only implements `{}`",
                            kind,
                            found_kind);

                        err.span_label(
                            closure_span,
                            format!("this closure implements `{}`, not `{}`", found_kind, kind));
                        err.span_label(
                            obligation.cause.span,
                            format!("the requirement to implement `{}` derives from here", kind));

                        // Additional context information explaining why the closure only implements
                        // a particular trait.
                        if let Some(tables) = self.in_progress_tables {
                            let tables = tables.borrow();
                            let closure_hir_id = self.tcx.hir.node_to_hir_id(node_id);
                            match (found_kind, tables.closure_kind_origins().get(closure_hir_id)) {
                                (ty::ClosureKind::FnOnce, Some((span, name))) => {
                                    err.span_label(*span, format!(
                                        "closure is `FnOnce` because it moves the \
                                         variable `{}` out of its environment", name));
                                },
                                (ty::ClosureKind::FnMut, Some((span, name))) => {
                                    err.span_label(*span, format!(
                                        "closure is `FnMut` because it mutates the \
                                         variable `{}` here", name));
                                },
                                _ => {}
                            }
                        }

                        err.emit();
                        return;
                    }

                    ty::Predicate::WellFormed(ty) => {
                        // WF predicates cannot themselves make
                        // errors. They can only block due to
                        // ambiguity; otherwise, they always
                        // degenerate into other obligations
                        // (which may fail).
                        span_bug!(span, "WF predicate not satisfied for {:?}", ty);
                    }

                    ty::Predicate::ConstEvaluatable(..) => {
                        // Errors for `ConstEvaluatable` predicates show up as
                        // `SelectionError::ConstEvalFailure`,
                        // not `Unimplemented`.
                        span_bug!(span,
                            "const-evaluatable requirement gave wrong error: `{:?}`", obligation)
                    }
                }
            }

            OutputTypeParameterMismatch(ref found_trait_ref, ref expected_trait_ref, _) => {
                let found_trait_ref = self.resolve_type_vars_if_possible(&*found_trait_ref);
                let expected_trait_ref = self.resolve_type_vars_if_possible(&*expected_trait_ref);
                if expected_trait_ref.self_ty().references_error() {
                    return;
                }
                let found_trait_ty = found_trait_ref.self_ty();

                let found_did = found_trait_ty.ty_to_def_id();
                let found_span = found_did.and_then(|did| {
                    self.tcx.hir.span_if_local(did)
                }).map(|sp| self.tcx.sess.codemap().def_span(sp)); // the sp could be an fn def

                let found = match found_trait_ref.skip_binder().substs.type_at(1).sty {
                    ty::TyTuple(ref tys) => tys.iter()
                        .map(|_| ArgKind::empty()).collect::<Vec<_>>(),
                    _ => vec![ArgKind::empty()],
                };
                let expected = match expected_trait_ref.skip_binder().substs.type_at(1).sty {
                    ty::TyTuple(ref tys) => tys.iter()
                        .map(|t| match t.sty {
                            ty::TypeVariants::TyTuple(ref tys) => ArgKind::Tuple(
                                Some(span),
                                tys.iter()
                                    .map(|ty| ("_".to_owned(), format!("{}", ty.sty)))
                                    .collect::<Vec<_>>()
                            ),
                            _ => ArgKind::Arg("_".to_owned(), format!("{}", t.sty)),
                        }).collect(),
                    ref sty => vec![ArgKind::Arg("_".to_owned(), format!("{}", sty))],
                };
                if found.len() == expected.len() {
                    self.report_closure_arg_mismatch(span,
                                                     found_span,
                                                     found_trait_ref,
                                                     expected_trait_ref)
                } else {
                    let (closure_span, found) = found_did
                        .and_then(|did| self.tcx.hir.get_if_local(did))
                        .map(|node| {
                            let (found_span, found) = self.get_fn_like_arguments(node);
                            (Some(found_span), found)
                        }).unwrap_or((found_span, found));

                    self.report_arg_count_mismatch(span,
                                                   closure_span,
                                                   expected,
                                                   found,
                                                   found_trait_ty.is_closure())
                }
            }

            TraitNotObjectSafe(did) => {
                let violations = self.tcx.object_safety_violations(did);
                self.tcx.report_object_safety_error(span, did,
                                                    violations)
            }

            ConstEvalFailure(ref err) => {
                match err.struct_error(
                    self.tcx.at(span),
                    "could not evaluate constant expression",
                ) {
                    Some(err) => err,
                    None => return,
                }
            }

            Overflow => {
                bug!("overflow should be handled before the `report_selection_error` path");
            }
        };
        self.note_obligation_cause(&mut err, obligation);
        err.emit();
    }

    /// When encountering an assignment of an unsized trait, like `let x = ""[..];`, provide a
    /// suggestion to borrow the initializer in order to use have a slice instead.
    fn suggest_borrow_on_unsized_slice(&self,
                                       code: &ObligationCauseCode<'tcx>,
                                       err: &mut DiagnosticBuilder<'tcx>) {
        if let &ObligationCauseCode::VariableType(node_id) = code {
            let parent_node = self.tcx.hir.get_parent_node(node_id);
            if let Some(hir::map::NodeLocal(ref local)) = self.tcx.hir.find(parent_node) {
                if let Some(ref expr) = local.init {
                    if let hir::ExprIndex(_, _) = expr.node {
                        if let Ok(snippet) = self.tcx.sess.codemap().span_to_snippet(expr.span) {
                            err.span_suggestion_with_applicability(
                                expr.span,
                                "consider borrowing here",
                                format!("&{}", snippet),
                                Applicability::MachineApplicable
                            );
                        }
                    }
                }
            }
        }
    }

    /// Whenever references are used by mistake, like `for (i, e) in &vec.iter().enumerate()`,
    /// suggest removing these references until we reach a type that implements the trait.
    fn suggest_remove_reference(&self,
                                obligation: &PredicateObligation<'tcx>,
                                err: &mut DiagnosticBuilder<'tcx>,
                                trait_ref: &ty::Binder<ty::TraitRef<'tcx>>) {
        let trait_ref = trait_ref.skip_binder();
        let span = obligation.cause.span;

        if let Ok(snippet) = self.tcx.sess.codemap().span_to_snippet(span) {
            let refs_number = snippet.chars()
                .filter(|c| !c.is_whitespace())
                .take_while(|c| *c == '&')
                .count();

            let mut trait_type = trait_ref.self_ty();

            for refs_remaining in 0..refs_number {
                if let ty::TypeVariants::TyRef(_, t_type, _) = trait_type.sty {
                    trait_type = t_type;

                    let substs = self.tcx.mk_substs_trait(trait_type, &[]);
                    let new_trait_ref = ty::TraitRef::new(trait_ref.def_id, substs);
                    let new_obligation = Obligation::new(ObligationCause::dummy(),
                                                         obligation.param_env,
                                                         new_trait_ref.to_predicate());

                    if self.predicate_may_hold(&new_obligation) {
                        let sp = self.tcx.sess.codemap()
                            .span_take_while(span, |c| c.is_whitespace() || *c == '&');

                        let remove_refs = refs_remaining + 1;
                        let format_str = format!("consider removing {} leading `&`-references",
                                                 remove_refs);

                        err.span_suggestion_short_with_applicability(
                            sp, &format_str, String::from(""), Applicability::MachineApplicable
                        );
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    /// Given some node representing a fn-like thing in the HIR map,
    /// returns a span and `ArgKind` information that describes the
    /// arguments it expects. This can be supplied to
    /// `report_arg_count_mismatch`.
    pub fn get_fn_like_arguments(&self, node: hir::map::Node) -> (Span, Vec<ArgKind>) {
        match node {
            hir::map::NodeExpr(&hir::Expr {
                node: hir::ExprClosure(_, ref _decl, id, span, _),
                ..
            }) => {
                (self.tcx.sess.codemap().def_span(span), self.tcx.hir.body(id).arguments.iter()
                    .map(|arg| {
                        if let hir::Pat {
                            node: hir::PatKind::Tuple(args, _),
                            span,
                            ..
                        } = arg.pat.clone().into_inner() {
                            ArgKind::Tuple(
                                Some(span),
                                args.iter().map(|pat| {
                                    let snippet = self.tcx.sess.codemap()
                                        .span_to_snippet(pat.span).unwrap();
                                    (snippet, "_".to_owned())
                                }).collect::<Vec<_>>(),
                            )
                        } else {
                            let name = self.tcx.sess.codemap()
                                .span_to_snippet(arg.pat.span).unwrap();
                            ArgKind::Arg(name, "_".to_owned())
                        }
                    })
                    .collect::<Vec<ArgKind>>())
            }
            hir::map::NodeItem(&hir::Item {
                span,
                node: hir::ItemFn(ref decl, ..),
                ..
            }) |
            hir::map::NodeImplItem(&hir::ImplItem {
                span,
                node: hir::ImplItemKind::Method(hir::MethodSig { ref decl, .. }, _),
                ..
            }) |
            hir::map::NodeTraitItem(&hir::TraitItem {
                span,
                node: hir::TraitItemKind::Method(hir::MethodSig { ref decl, .. }, _),
                ..
            }) => {
                (self.tcx.sess.codemap().def_span(span), decl.inputs.iter()
                        .map(|arg| match arg.clone().into_inner().node {
                    hir::TyTup(ref tys) => ArgKind::Tuple(
                        Some(arg.span),
                        tys.iter()
                            .map(|_| ("_".to_owned(), "_".to_owned()))
                            .collect::<Vec<_>>(),
                    ),
                    _ => ArgKind::Arg("_".to_owned(), "_".to_owned())
                }).collect::<Vec<ArgKind>>())
            }
            hir::map::NodeVariant(&hir::Variant {
                span,
                node: hir::Variant_ {
                    data: hir::VariantData::Tuple(ref fields, _),
                    ..
                },
                ..
            }) => {
                (self.tcx.sess.codemap().def_span(span),
                 fields.iter().map(|field| {
                     ArgKind::Arg(format!("{}", field.ident), "_".to_string())
                 }).collect::<Vec<_>>())
            }
            hir::map::NodeStructCtor(ref variant_data) => {
                (self.tcx.sess.codemap().def_span(self.tcx.hir.span(variant_data.id())),
                 variant_data.fields()
                    .iter().map(|_| ArgKind::Arg("_".to_owned(), "_".to_owned()))
                    .collect())
            }
            _ => panic!("non-FnLike node found: {:?}", node),
        }
    }

    /// Reports an error when the number of arguments needed by a
    /// trait match doesn't match the number that the expression
    /// provides.
    pub fn report_arg_count_mismatch(
        &self,
        span: Span,
        found_span: Option<Span>,
        expected_args: Vec<ArgKind>,
        found_args: Vec<ArgKind>,
        is_closure: bool,
    ) -> DiagnosticBuilder<'tcx> {
        let kind = if is_closure { "closure" } else { "function" };

        let args_str = |arguments: &Vec<ArgKind>, other: &Vec<ArgKind>| {
            let arg_length = arguments.len();
            let distinct = match &other[..] {
                &[ArgKind::Tuple(..)] => true,
                _ => false,
            };
            match (arg_length, arguments.get(0)) {
                (1, Some(&ArgKind::Tuple(_, ref fields))) => {
                    format!("a single {}-tuple as argument", fields.len())
                }
                _ => format!("{} {}argument{}",
                             arg_length,
                             if distinct && arg_length > 1 { "distinct " } else { "" },
                             if arg_length == 1 { "" } else { "s" }),
            }
        };

        let expected_str = args_str(&expected_args, &found_args);
        let found_str = args_str(&found_args, &expected_args);

        let mut err = struct_span_err!(
            self.tcx.sess,
            span,
            E0593,
            "{} is expected to take {}, but it takes {}",
            kind,
            expected_str,
            found_str,
        );

        err.span_label(span, format!( "expected {} that takes {}", kind, expected_str));

        if let Some(found_span) = found_span {
            err.span_label(found_span, format!("takes {}", found_str));

            if let &[ArgKind::Tuple(_, ref fields)] = &found_args[..] {
                if fields.len() == expected_args.len() {
                    let sugg = fields.iter()
                        .map(|(name, _)| name.to_owned())
                        .collect::<Vec<String>>().join(", ");
                    err.span_suggestion_with_applicability(found_span,
                                                           "change the closure to take multiple \
                                                            arguments instead of a single tuple",
                                                           format!("|{}|", sugg),
                                                           Applicability::MachineApplicable);
                }
            }
            if let &[ArgKind::Tuple(_, ref fields)] = &expected_args[..] {
                if fields.len() == found_args.len() && is_closure {
                    let sugg = format!(
                        "|({}){}|",
                        found_args.iter()
                            .map(|arg| match arg {
                                ArgKind::Arg(name, _) => name.to_owned(),
                                _ => "_".to_owned(),
                            })
                            .collect::<Vec<String>>()
                            .join(", "),
                        // add type annotations if available
                        if found_args.iter().any(|arg| match arg {
                            ArgKind::Arg(_, ty) => ty != "_",
                            _ => false,
                        }) {
                            format!(": ({})",
                                    fields.iter()
                                        .map(|(_, ty)| ty.to_owned())
                                        .collect::<Vec<String>>()
                                        .join(", "))
                        } else {
                            "".to_owned()
                        },
                    );
                    err.span_suggestion_with_applicability(
                        found_span,
                        "change the closure to accept a tuple instead of \
                         individual arguments",
                        sugg,
                        Applicability::MachineApplicable
                    );
                }
            }
        }

        err
    }

    fn report_closure_arg_mismatch(&self,
                           span: Span,
                           found_span: Option<Span>,
                           expected_ref: ty::PolyTraitRef<'tcx>,
                           found: ty::PolyTraitRef<'tcx>)
        -> DiagnosticBuilder<'tcx>
    {
        fn build_fn_sig_string<'a, 'gcx, 'tcx>(tcx: ty::TyCtxt<'a, 'gcx, 'tcx>,
                                               trait_ref: &ty::TraitRef<'tcx>) -> String {
            let inputs = trait_ref.substs.type_at(1);
            let sig = if let ty::TyTuple(inputs) = inputs.sty {
                tcx.mk_fn_sig(
                    inputs.iter().map(|&x| x),
                    tcx.mk_infer(ty::TyVar(ty::TyVid { index: 0 })),
                    false,
                    hir::Unsafety::Normal,
                    ::rustc_target::spec::abi::Abi::Rust
                )
            } else {
                tcx.mk_fn_sig(
                    ::std::iter::once(inputs),
                    tcx.mk_infer(ty::TyVar(ty::TyVid { index: 0 })),
                    false,
                    hir::Unsafety::Normal,
                    ::rustc_target::spec::abi::Abi::Rust
                )
            };
            format!("{}", ty::Binder::bind(sig))
        }

        let argument_is_closure = expected_ref.skip_binder().substs.type_at(0).is_closure();
        let mut err = struct_span_err!(self.tcx.sess, span, E0631,
                                       "type mismatch in {} arguments",
                                       if argument_is_closure { "closure" } else { "function" });

        let found_str = format!(
            "expected signature of `{}`",
            build_fn_sig_string(self.tcx, found.skip_binder())
        );
        err.span_label(span, found_str);

        let found_span = found_span.unwrap_or(span);
        let expected_str = format!(
            "found signature of `{}`",
            build_fn_sig_string(self.tcx, expected_ref.skip_binder())
        );
        err.span_label(found_span, expected_str);

        err
    }
}

impl<'a, 'gcx, 'tcx> TyCtxt<'a, 'gcx, 'tcx> {
    pub fn recursive_type_with_infinite_size_error(self,
                                                   type_def_id: DefId)
                                                   -> DiagnosticBuilder<'tcx>
    {
        assert!(type_def_id.is_local());
        let span = self.hir.span_if_local(type_def_id).unwrap();
        let span = self.sess.codemap().def_span(span);
        let mut err = struct_span_err!(self.sess, span, E0072,
                                       "recursive type `{}` has infinite size",
                                       self.item_path_str(type_def_id));
        err.span_label(span, "recursive type has infinite size");
        err.help(&format!("insert indirection (e.g., a `Box`, `Rc`, or `&`) \
                           at some point to make `{}` representable",
                          self.item_path_str(type_def_id)));
        err
    }

    pub fn report_object_safety_error(self,
                                      span: Span,
                                      trait_def_id: DefId,
                                      violations: Vec<ObjectSafetyViolation>)
                                      -> DiagnosticBuilder<'tcx>
    {
        let trait_str = self.item_path_str(trait_def_id);
        let span = self.sess.codemap().def_span(span);
        let mut err = struct_span_err!(
            self.sess, span, E0038,
            "the trait `{}` cannot be made into an object",
            trait_str);
        err.span_label(span, format!("the trait `{}` cannot be made into an object", trait_str));

        let mut reported_violations = FxHashSet();
        for violation in violations {
            if !reported_violations.insert(violation.clone()) {
                continue;
            }
            err.note(&violation.error_msg());
        }
        err
    }
}

impl<'a, 'gcx, 'tcx> InferCtxt<'a, 'gcx, 'tcx> {
    fn maybe_report_ambiguity(&self, obligation: &PredicateObligation<'tcx>,
                              body_id: Option<hir::BodyId>) {
        // Unable to successfully determine, probably means
        // insufficient type information, but could mean
        // ambiguous impls. The latter *ought* to be a
        // coherence violation, so we don't report it here.

        let predicate = self.resolve_type_vars_if_possible(&obligation.predicate);
        let span = obligation.cause.span;

        debug!("maybe_report_ambiguity(predicate={:?}, obligation={:?})",
               predicate,
               obligation);

        // Ambiguity errors are often caused as fallout from earlier
        // errors. So just ignore them if this infcx is tainted.
        if self.is_tainted_by_errors() {
            return;
        }

        match predicate {
            ty::Predicate::Trait(ref data) => {
                let trait_ref = data.to_poly_trait_ref();
                let self_ty = trait_ref.self_ty();
                if predicate.references_error() {
                    return;
                }
                // Typically, this ambiguity should only happen if
                // there are unresolved type inference variables
                // (otherwise it would suggest a coherence
                // failure). But given #21974 that is not necessarily
                // the case -- we can have multiple where clauses that
                // are only distinguished by a region, which results
                // in an ambiguity even when all types are fully
                // known, since we don't dispatch based on region
                // relationships.

                // This is kind of a hack: it frequently happens that some earlier
                // error prevents types from being fully inferred, and then we get
                // a bunch of uninteresting errors saying something like "<generic
                // #0> doesn't implement Sized".  It may even be true that we
                // could just skip over all checks where the self-ty is an
                // inference variable, but I was afraid that there might be an
                // inference variable created, registered as an obligation, and
                // then never forced by writeback, and hence by skipping here we'd
                // be ignoring the fact that we don't KNOW the type works
                // out. Though even that would probably be harmless, given that
                // we're only talking about builtin traits, which are known to be
                // inhabited. But in any case I just threw in this check for
                // has_errors() to be sure that compilation isn't happening
                // anyway. In that case, why inundate the user.
                if !self.tcx.sess.has_errors() {
                    if
                        self.tcx.lang_items().sized_trait()
                        .map_or(false, |sized_id| sized_id == trait_ref.def_id())
                    {
                        self.need_type_info_err(body_id, span, self_ty).emit();
                    } else {
                        let mut err = struct_span_err!(self.tcx.sess,
                                                        span, E0283,
                                                        "type annotations required: \
                                                        cannot resolve `{}`",
                                                        predicate);
                        self.note_obligation_cause(&mut err, obligation);
                        err.emit();
                    }
                }
            }

            ty::Predicate::WellFormed(ty) => {
                // Same hacky approach as above to avoid deluging user
                // with error messages.
                if !ty.references_error() && !self.tcx.sess.has_errors() {
                    self.need_type_info_err(body_id, span, ty).emit();
                }
            }

            ty::Predicate::Subtype(ref data) => {
                if data.references_error() || self.tcx.sess.has_errors() {
                    // no need to overload user in such cases
                } else {
                    let &SubtypePredicate { a_is_expected: _, a, b } = data.skip_binder();
                    // both must be type variables, or the other would've been instantiated
                    assert!(a.is_ty_var() && b.is_ty_var());
                    self.need_type_info_err(body_id,
                                            obligation.cause.span,
                                            a).emit();
                }
            }

            _ => {
                if !self.tcx.sess.has_errors() {
                    let mut err = struct_span_err!(self.tcx.sess,
                                                   obligation.cause.span, E0284,
                                                   "type annotations required: \
                                                    cannot resolve `{}`",
                                                   predicate);
                    self.note_obligation_cause(&mut err, obligation);
                    err.emit();
                }
            }
        }
    }

    /// Returns whether the trait predicate may apply for *some* assignment
    /// to the type parameters.
    fn predicate_can_apply(&self,
                           param_env: ty::ParamEnv<'tcx>,
                           pred: ty::PolyTraitRef<'tcx>)
                           -> bool {
        struct ParamToVarFolder<'a, 'gcx: 'a+'tcx, 'tcx: 'a> {
            infcx: &'a InferCtxt<'a, 'gcx, 'tcx>,
            var_map: FxHashMap<Ty<'tcx>, Ty<'tcx>>
        }

        impl<'a, 'gcx, 'tcx> TypeFolder<'gcx, 'tcx> for ParamToVarFolder<'a, 'gcx, 'tcx> {
            fn tcx<'b>(&'b self) -> TyCtxt<'b, 'gcx, 'tcx> { self.infcx.tcx }

            fn fold_ty(&mut self, ty: Ty<'tcx>) -> Ty<'tcx> {
                if let ty::TyParam(ty::ParamTy {name, ..}) = ty.sty {
                    let infcx = self.infcx;
                    self.var_map.entry(ty).or_insert_with(||
                        infcx.next_ty_var(
                            TypeVariableOrigin::TypeParameterDefinition(DUMMY_SP, name)))
                } else {
                    ty.super_fold_with(self)
                }
            }
        }

        self.probe(|_| {
            let mut selcx = SelectionContext::new(self);

            let cleaned_pred = pred.fold_with(&mut ParamToVarFolder {
                infcx: self,
                var_map: FxHashMap()
            });

            let cleaned_pred = super::project::normalize(
                &mut selcx,
                param_env,
                ObligationCause::dummy(),
                &cleaned_pred
            ).value;

            let obligation = Obligation::new(
                ObligationCause::dummy(),
                param_env,
                cleaned_pred.to_predicate()
            );

            self.predicate_may_hold(&obligation)
        })
    }

    fn note_obligation_cause<T>(&self,
                                err: &mut DiagnosticBuilder,
                                obligation: &Obligation<'tcx, T>)
        where T: fmt::Display
    {
        self.note_obligation_cause_code(err,
                                        &obligation.predicate,
                                        &obligation.cause.code,
                                        &mut vec![]);
    }

    fn note_obligation_cause_code<T>(&self,
                                     err: &mut DiagnosticBuilder,
                                     predicate: &T,
                                     cause_code: &ObligationCauseCode<'tcx>,
                                     obligated_types: &mut Vec<&ty::TyS<'tcx>>)
        where T: fmt::Display
    {
        let tcx = self.tcx;
        match *cause_code {
            ObligationCauseCode::ExprAssignable |
            ObligationCauseCode::MatchExpressionArm { .. } |
            ObligationCauseCode::IfExpression |
            ObligationCauseCode::IfExpressionWithNoElse |
            ObligationCauseCode::MainFunctionType |
            ObligationCauseCode::StartFunctionType |
            ObligationCauseCode::IntrinsicType |
            ObligationCauseCode::MethodReceiver |
            ObligationCauseCode::ReturnNoExpression |
            ObligationCauseCode::MiscObligation => {
            }
            ObligationCauseCode::SliceOrArrayElem => {
                err.note("slice and array elements must have `Sized` type");
            }
            ObligationCauseCode::TupleElem => {
                err.note("only the last element of a tuple may have a dynamically sized type");
            }
            ObligationCauseCode::ProjectionWf(data) => {
                err.note(&format!("required so that the projection `{}` is well-formed",
                                  data));
            }
            ObligationCauseCode::ReferenceOutlivesReferent(ref_ty) => {
                err.note(&format!("required so that reference `{}` does not outlive its referent",
                                  ref_ty));
            }
            ObligationCauseCode::ObjectTypeBound(object_ty, region) => {
                err.note(&format!("required so that the lifetime bound of `{}` for `{}` \
                                   is satisfied",
                                  region, object_ty));
            }
            ObligationCauseCode::ItemObligation(item_def_id) => {
                let item_name = tcx.item_path_str(item_def_id);
                let msg = format!("required by `{}`", item_name);
                if let Some(sp) = tcx.hir.span_if_local(item_def_id) {
                    let sp = tcx.sess.codemap().def_span(sp);
                    err.span_note(sp, &msg);
                } else {
                    err.note(&msg);
                }
            }
            ObligationCauseCode::ObjectCastObligation(object_ty) => {
                err.note(&format!("required for the cast to the object type `{}`",
                                  self.ty_to_string(object_ty)));
            }
            ObligationCauseCode::RepeatVec => {
                err.note("the `Copy` trait is required because the \
                          repeated element will be copied");
            }
            ObligationCauseCode::VariableType(_) => {
                err.note("all local variables must have a statically known size");
            }
            ObligationCauseCode::SizedReturnType => {
                err.note("the return type of a function must have a \
                          statically known size");
            }
            ObligationCauseCode::SizedYieldType => {
                err.note("the yield type of a generator must have a \
                          statically known size");
            }
            ObligationCauseCode::AssignmentLhsSized => {
                err.note("the left-hand-side of an assignment must have a statically known size");
            }
            ObligationCauseCode::TupleInitializerSized => {
                err.note("tuples must have a statically known size to be initialized");
            }
            ObligationCauseCode::StructInitializerSized => {
                err.note("structs must have a statically known size to be initialized");
            }
            ObligationCauseCode::FieldSized(ref item) => {
                match *item {
                    AdtKind::Struct => {
                        err.note("only the last field of a struct may have a dynamically \
                                  sized type");
                    }
                    AdtKind::Union => {
                        err.note("no field of a union may have a dynamically sized type");
                    }
                    AdtKind::Enum => {
                        err.note("no field of an enum variant may have a dynamically sized type");
                    }
                }
            }
            ObligationCauseCode::ConstSized => {
                err.note("constant expressions must have a statically known size");
            }
            ObligationCauseCode::SharedStatic => {
                err.note("shared static variables must have a type that implements `Sync`");
            }
            ObligationCauseCode::BuiltinDerivedObligation(ref data) => {
                let parent_trait_ref = self.resolve_type_vars_if_possible(&data.parent_trait_ref);
                let ty = parent_trait_ref.skip_binder().self_ty();
                err.note(&format!("required because it appears within the type `{}`", ty));
                obligated_types.push(ty);

                let parent_predicate = parent_trait_ref.to_predicate();
                if !self.is_recursive_obligation(obligated_types, &data.parent_code) {
                    self.note_obligation_cause_code(err,
                                                    &parent_predicate,
                                                    &data.parent_code,
                                                    obligated_types);
                }
            }
            ObligationCauseCode::ImplDerivedObligation(ref data) => {
                let parent_trait_ref = self.resolve_type_vars_if_possible(&data.parent_trait_ref);
                err.note(
                    &format!("required because of the requirements on the impl of `{}` for `{}`",
                             parent_trait_ref,
                             parent_trait_ref.skip_binder().self_ty()));
                let parent_predicate = parent_trait_ref.to_predicate();
                self.note_obligation_cause_code(err,
                                            &parent_predicate,
                                            &data.parent_code,
                                            obligated_types);
            }
            ObligationCauseCode::CompareImplMethodObligation { .. } => {
                err.note(
                    &format!("the requirement `{}` appears on the impl method \
                              but not on the corresponding trait method",
                             predicate));
            }
            ObligationCauseCode::ReturnType(_) |
            ObligationCauseCode::BlockTailExpression(_) => (),
            ObligationCauseCode::TrivialBound => {
                err.help("see issue #48214");
                if tcx.sess.opts.unstable_features.is_nightly_build() {
                    err.help("add #![feature(trivial_bounds)] to the \
                              crate attributes to enable",
                    );
                }
            }
        }
    }

    fn suggest_new_overflow_limit(&self, err: &mut DiagnosticBuilder) {
        let current_limit = self.tcx.sess.recursion_limit.get();
        let suggested_limit = current_limit * 2;
        err.help(&format!("consider adding a `#![recursion_limit=\"{}\"]` attribute to your crate",
                          suggested_limit));
    }

    fn is_recursive_obligation(&self,
                                   obligated_types: &mut Vec<&ty::TyS<'tcx>>,
                                   cause_code: &ObligationCauseCode<'tcx>) -> bool {
        if let ObligationCauseCode::BuiltinDerivedObligation(ref data) = cause_code {
            let parent_trait_ref = self.resolve_type_vars_if_possible(&data.parent_trait_ref);
            for obligated_type in obligated_types {
                if obligated_type == &parent_trait_ref.skip_binder().self_ty() {
                    return true;
                }
            }
        }
        return false;
    }
}

/// Summarizes information
pub enum ArgKind {
    /// An argument of non-tuple type. Parameters are (name, ty)
    Arg(String, String),

    /// An argument of tuple type. For a "found" argument, the span is
    /// the locationo in the source of the pattern. For a "expected"
    /// argument, it will be None. The vector is a list of (name, ty)
    /// strings for the components of the tuple.
    Tuple(Option<Span>, Vec<(String, String)>),
}

impl ArgKind {
    fn empty() -> ArgKind {
        ArgKind::Arg("_".to_owned(), "_".to_owned())
    }

    /// Creates an `ArgKind` from the expected type of an
    /// argument. This has no name (`_`) and no source spans..
    pub fn from_expected_ty(t: Ty<'_>) -> ArgKind {
        match t.sty {
            ty::TyTuple(ref tys) => ArgKind::Tuple(
                None,
                tys.iter()
                   .map(|ty| ("_".to_owned(), format!("{}", ty.sty)))
                   .collect::<Vec<_>>()
            ),
            _ => ArgKind::Arg("_".to_owned(), format!("{}", t.sty)),
        }
    }
}
