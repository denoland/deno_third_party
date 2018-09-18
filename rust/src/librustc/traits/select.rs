// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! See [rustc guide] for more info on how this works.
//!
//! [rustc guide]: https://rust-lang-nursery.github.io/rustc-guide/trait-resolution.html#selection

use self::SelectionCandidate::*;
use self::EvaluationResult::*;

use super::coherence::{self, Conflict};
use super::DerivedObligationCause;
use super::{IntercrateMode, TraitQueryMode};
use super::project;
use super::project::{normalize_with_depth, Normalized, ProjectionCacheKey};
use super::{PredicateObligation, TraitObligation, ObligationCause};
use super::{ObligationCauseCode, BuiltinDerivedObligation, ImplDerivedObligation};
use super::{SelectionError, Unimplemented, OutputTypeParameterMismatch, Overflow};
use super::{ObjectCastObligation, Obligation};
use super::TraitNotObjectSafe;
use super::Selection;
use super::SelectionResult;
use super::{VtableBuiltin, VtableImpl, VtableParam, VtableClosure, VtableGenerator,
            VtableFnPointer, VtableObject, VtableAutoImpl};
use super::{VtableImplData, VtableObjectData, VtableBuiltinData, VtableGeneratorData,
            VtableClosureData, VtableAutoImplData, VtableFnPointerData};
use super::util;

use dep_graph::{DepNodeIndex, DepKind};
use hir::def_id::DefId;
use infer;
use infer::{InferCtxt, InferOk, TypeFreshener};
use ty::subst::{Subst, Substs};
use ty::{self, ToPredicate, ToPolyTraitRef, Ty, TyCtxt, TypeFoldable};
use ty::fast_reject;
use ty::relate::TypeRelation;
use middle::lang_items;
use mir::interpret::{GlobalId};

use rustc_data_structures::sync::Lock;
use rustc_data_structures::bitvec::BitVector;
use std::iter;
use std::cmp;
use std::fmt;
use std::mem;
use std::rc::Rc;
use rustc_target::spec::abi::Abi;
use hir;
use util::nodemap::{FxHashMap, FxHashSet};


pub struct SelectionContext<'cx, 'gcx: 'cx+'tcx, 'tcx: 'cx> {
    infcx: &'cx InferCtxt<'cx, 'gcx, 'tcx>,

    /// Freshener used specifically for skolemizing entries on the
    /// obligation stack. This ensures that all entries on the stack
    /// at one time will have the same set of skolemized entries,
    /// which is important for checking for trait bounds that
    /// recursively require themselves.
    freshener: TypeFreshener<'cx, 'gcx, 'tcx>,

    /// If true, indicates that the evaluation should be conservative
    /// and consider the possibility of types outside this crate.
    /// This comes up primarily when resolving ambiguity. Imagine
    /// there is some trait reference `$0 : Bar` where `$0` is an
    /// inference variable. If `intercrate` is true, then we can never
    /// say for sure that this reference is not implemented, even if
    /// there are *no impls at all for `Bar`*, because `$0` could be
    /// bound to some type that in a downstream crate that implements
    /// `Bar`. This is the suitable mode for coherence. Elsewhere,
    /// though, we set this to false, because we are only interested
    /// in types that the user could actually have written --- in
    /// other words, we consider `$0 : Bar` to be unimplemented if
    /// there is no type that the user could *actually name* that
    /// would satisfy it. This avoids crippling inference, basically.
    intercrate: Option<IntercrateMode>,

    intercrate_ambiguity_causes: Option<Vec<IntercrateAmbiguityCause>>,

    /// Controls whether or not to filter out negative impls when selecting.
    /// This is used in librustdoc to distinguish between the lack of an impl
    /// and a negative impl
    allow_negative_impls: bool,

    /// The mode that trait queries run in, which informs our error handling
    /// policy. In essence, canonicalized queries need their errors propagated
    /// rather than immediately reported because we do not have accurate spans.
    query_mode: TraitQueryMode,
}

#[derive(Clone, Debug)]
pub enum IntercrateAmbiguityCause {
    DownstreamCrate {
        trait_desc: String,
        self_desc: Option<String>,
    },
    UpstreamCrateUpdate {
        trait_desc: String,
        self_desc: Option<String>,
    },
}

impl IntercrateAmbiguityCause {
    /// Emits notes when the overlap is caused by complex intercrate ambiguities.
    /// See #23980 for details.
    pub fn add_intercrate_ambiguity_hint<'a, 'tcx>(&self,
                                                   err: &mut ::errors::DiagnosticBuilder) {
        err.note(&self.intercrate_ambiguity_hint());
    }

    pub fn intercrate_ambiguity_hint(&self) -> String {
        match self {
            &IntercrateAmbiguityCause::DownstreamCrate { ref trait_desc, ref self_desc } => {
                let self_desc = if let &Some(ref ty) = self_desc {
                    format!(" for type `{}`", ty)
                } else { "".to_string() };
                format!("downstream crates may implement trait `{}`{}", trait_desc, self_desc)
            }
            &IntercrateAmbiguityCause::UpstreamCrateUpdate { ref trait_desc, ref self_desc } => {
                let self_desc = if let &Some(ref ty) = self_desc {
                    format!(" for type `{}`", ty)
                } else { "".to_string() };
                format!("upstream crates may add new impl of trait `{}`{} \
                         in future versions",
                        trait_desc, self_desc)
            }
        }
    }
}

// A stack that walks back up the stack frame.
struct TraitObligationStack<'prev, 'tcx: 'prev> {
    obligation: &'prev TraitObligation<'tcx>,

    /// Trait ref from `obligation` but skolemized with the
    /// selection-context's freshener. Used to check for recursion.
    fresh_trait_ref: ty::PolyTraitRef<'tcx>,

    previous: TraitObligationStackList<'prev, 'tcx>,
}

#[derive(Clone)]
pub struct SelectionCache<'tcx> {
    hashmap: Lock<FxHashMap<ty::TraitRef<'tcx>,
                               WithDepNode<SelectionResult<'tcx, SelectionCandidate<'tcx>>>>>,
}

/// The selection process begins by considering all impls, where
/// clauses, and so forth that might resolve an obligation.  Sometimes
/// we'll be able to say definitively that (e.g.) an impl does not
/// apply to the obligation: perhaps it is defined for `usize` but the
/// obligation is for `int`. In that case, we drop the impl out of the
/// list.  But the other cases are considered *candidates*.
///
/// For selection to succeed, there must be exactly one matching
/// candidate. If the obligation is fully known, this is guaranteed
/// by coherence. However, if the obligation contains type parameters
/// or variables, there may be multiple such impls.
///
/// It is not a real problem if multiple matching impls exist because
/// of type variables - it just means the obligation isn't sufficiently
/// elaborated. In that case we report an ambiguity, and the caller can
/// try again after more type information has been gathered or report a
/// "type annotations required" error.
///
/// However, with type parameters, this can be a real problem - type
/// parameters don't unify with regular types, but they *can* unify
/// with variables from blanket impls, and (unless we know its bounds
/// will always be satisfied) picking the blanket impl will be wrong
/// for at least *some* substitutions. To make this concrete, if we have
///
///    trait AsDebug { type Out : fmt::Debug; fn debug(self) -> Self::Out; }
///    impl<T: fmt::Debug> AsDebug for T {
///        type Out = T;
///        fn debug(self) -> fmt::Debug { self }
///    }
///    fn foo<T: AsDebug>(t: T) { println!("{:?}", <T as AsDebug>::debug(t)); }
///
/// we can't just use the impl to resolve the <T as AsDebug> obligation
/// - a type from another crate (that doesn't implement fmt::Debug) could
/// implement AsDebug.
///
/// Because where-clauses match the type exactly, multiple clauses can
/// only match if there are unresolved variables, and we can mostly just
/// report this ambiguity in that case. This is still a problem - we can't
/// *do anything* with ambiguities that involve only regions. This is issue
/// #21974.
///
/// If a single where-clause matches and there are no inference
/// variables left, then it definitely matches and we can just select
/// it.
///
/// In fact, we even select the where-clause when the obligation contains
/// inference variables. The can lead to inference making "leaps of logic",
/// for example in this situation:
///
///    pub trait Foo<T> { fn foo(&self) -> T; }
///    impl<T> Foo<()> for T { fn foo(&self) { } }
///    impl Foo<bool> for bool { fn foo(&self) -> bool { *self } }
///
///    pub fn foo<T>(t: T) where T: Foo<bool> {
///       println!("{:?}", <T as Foo<_>>::foo(&t));
///    }
///    fn main() { foo(false); }
///
/// Here the obligation <T as Foo<$0>> can be matched by both the blanket
/// impl and the where-clause. We select the where-clause and unify $0=bool,
/// so the program prints "false". However, if the where-clause is omitted,
/// the blanket impl is selected, we unify $0=(), and the program prints
/// "()".
///
/// Exactly the same issues apply to projection and object candidates, except
/// that we can have both a projection candidate and a where-clause candidate
/// for the same obligation. In that case either would do (except that
/// different "leaps of logic" would occur if inference variables are
/// present), and we just pick the where-clause. This is, for example,
/// required for associated types to work in default impls, as the bounds
/// are visible both as projection bounds and as where-clauses from the
/// parameter environment.
#[derive(PartialEq,Eq,Debug,Clone)]
enum SelectionCandidate<'tcx> {
    BuiltinCandidate { has_nested: bool },
    ParamCandidate(ty::PolyTraitRef<'tcx>),
    ImplCandidate(DefId),
    AutoImplCandidate(DefId),

    /// This is a trait matching with a projected type as `Self`, and
    /// we found an applicable bound in the trait definition.
    ProjectionCandidate,

    /// Implementation of a `Fn`-family trait by one of the anonymous types
    /// generated for a `||` expression.
    ClosureCandidate,

    /// Implementation of a `Generator` trait by one of the anonymous types
    /// generated for a generator.
    GeneratorCandidate,

    /// Implementation of a `Fn`-family trait by one of the anonymous
    /// types generated for a fn pointer type (e.g., `fn(int)->int`)
    FnPointerCandidate,

    ObjectCandidate,

    BuiltinObjectCandidate,

    BuiltinUnsizeCandidate,
}

impl<'a, 'tcx> ty::Lift<'tcx> for SelectionCandidate<'a> {
    type Lifted = SelectionCandidate<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        Some(match *self {
            BuiltinCandidate { has_nested } => {
                BuiltinCandidate {
                    has_nested,
                }
            }
            ImplCandidate(def_id) => ImplCandidate(def_id),
            AutoImplCandidate(def_id) => AutoImplCandidate(def_id),
            ProjectionCandidate => ProjectionCandidate,
            FnPointerCandidate => FnPointerCandidate,
            ObjectCandidate => ObjectCandidate,
            BuiltinObjectCandidate => BuiltinObjectCandidate,
            BuiltinUnsizeCandidate => BuiltinUnsizeCandidate,
            ClosureCandidate => ClosureCandidate,
            GeneratorCandidate => GeneratorCandidate,

            ParamCandidate(ref trait_ref) => {
                return tcx.lift(trait_ref).map(ParamCandidate);
            }
        })
    }
}

struct SelectionCandidateSet<'tcx> {
    // a list of candidates that definitely apply to the current
    // obligation (meaning: types unify).
    vec: Vec<SelectionCandidate<'tcx>>,

    // if this is true, then there were candidates that might or might
    // not have applied, but we couldn't tell. This occurs when some
    // of the input types are type variables, in which case there are
    // various "builtin" rules that might or might not trigger.
    ambiguous: bool,
}

#[derive(PartialEq,Eq,Debug,Clone)]
struct EvaluatedCandidate<'tcx> {
    candidate: SelectionCandidate<'tcx>,
    evaluation: EvaluationResult,
}

/// When does the builtin impl for `T: Trait` apply?
enum BuiltinImplConditions<'tcx> {
    /// The impl is conditional on T1,T2,.. : Trait
    Where(ty::Binder<Vec<Ty<'tcx>>>),
    /// There is no built-in impl. There may be some other
    /// candidate (a where-clause or user-defined impl).
    None,
    /// It is unknown whether there is an impl.
    Ambiguous
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
/// The result of trait evaluation. The order is important
/// here as the evaluation of a list is the maximum of the
/// evaluations.
///
/// The evaluation results are ordered:
///     - `EvaluatedToOk` implies `EvaluatedToAmbig` implies `EvaluatedToUnknown`
///     - `EvaluatedToErr` implies `EvaluatedToRecur`
///     - the "union" of evaluation results is equal to their maximum -
///     all the "potential success" candidates can potentially succeed,
///     so they are no-ops when unioned with a definite error, and within
///     the categories it's easy to see that the unions are correct.
pub enum EvaluationResult {
    /// Evaluation successful
    EvaluatedToOk,
    /// Evaluation is known to be ambiguous - it *might* hold for some
    /// assignment of inference variables, but it might not.
    ///
    /// While this has the same meaning as `EvaluatedToUnknown` - we can't
    /// know whether this obligation holds or not - it is the result we
    /// would get with an empty stack, and therefore is cacheable.
    EvaluatedToAmbig,
    /// Evaluation failed because of recursion involving inference
    /// variables. We are somewhat imprecise there, so we don't actually
    /// know the real result.
    ///
    /// This can't be trivially cached for the same reason as `EvaluatedToRecur`.
    EvaluatedToUnknown,
    /// Evaluation failed because we encountered an obligation we are already
    /// trying to prove on this branch.
    ///
    /// We know this branch can't be a part of a minimal proof-tree for
    /// the "root" of our cycle, because then we could cut out the recursion
    /// and maintain a valid proof tree. However, this does not mean
    /// that all the obligations on this branch do not hold - it's possible
    /// that we entered this branch "speculatively", and that there
    /// might be some other way to prove this obligation that does not
    /// go through this cycle - so we can't cache this as a failure.
    ///
    /// For example, suppose we have this:
    ///
    /// ```rust,ignore (pseudo-Rust)
    ///     pub trait Trait { fn xyz(); }
    ///     // This impl is "useless", but we can still have
    ///     // an `impl Trait for SomeUnsizedType` somewhere.
    ///     impl<T: Trait + Sized> Trait for T { fn xyz() {} }
    ///
    ///     pub fn foo<T: Trait + ?Sized>() {
    ///         <T as Trait>::xyz();
    ///     }
    /// ```
    ///
    /// When checking `foo`, we have to prove `T: Trait`. This basically
    /// translates into this:
    ///
    /// ```plain,ignore
    ///     (T: Trait + Sized →_\impl T: Trait), T: Trait ⊢ T: Trait
    /// ```
    ///
    /// When we try to prove it, we first go the first option, which
    /// recurses. This shows us that the impl is "useless" - it won't
    /// tell us that `T: Trait` unless it already implemented `Trait`
    /// by some other means. However, that does not prevent `T: Trait`
    /// does not hold, because of the bound (which can indeed be satisfied
    /// by `SomeUnsizedType` from another crate).
    ///
    /// FIXME: when an `EvaluatedToRecur` goes past its parent root, we
    /// ought to convert it to an `EvaluatedToErr`, because we know
    /// there definitely isn't a proof tree for that obligation. Not
    /// doing so is still sound - there isn't any proof tree, so the
    /// branch still can't be a part of a minimal one - but does not
    /// re-enable caching.
    EvaluatedToRecur,
    /// Evaluation failed
    EvaluatedToErr,
}

impl EvaluationResult {
    pub fn may_apply(self) -> bool {
        match self {
            EvaluatedToOk |
            EvaluatedToAmbig |
            EvaluatedToUnknown => true,

            EvaluatedToErr |
            EvaluatedToRecur => false
        }
    }

    fn is_stack_dependent(self) -> bool {
        match self {
            EvaluatedToUnknown |
            EvaluatedToRecur => true,

            EvaluatedToOk |
            EvaluatedToAmbig |
            EvaluatedToErr => false,
        }
    }
}

impl_stable_hash_for!(enum self::EvaluationResult {
    EvaluatedToOk,
    EvaluatedToAmbig,
    EvaluatedToUnknown,
    EvaluatedToRecur,
    EvaluatedToErr
});

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Indicates that trait evaluation caused overflow.
pub struct OverflowError;

impl_stable_hash_for!(struct OverflowError { });

impl<'tcx> From<OverflowError> for SelectionError<'tcx> {
    fn from(OverflowError: OverflowError) -> SelectionError<'tcx> {
        SelectionError::Overflow
    }
}

#[derive(Clone)]
pub struct EvaluationCache<'tcx> {
    hashmap: Lock<FxHashMap<ty::PolyTraitRef<'tcx>, WithDepNode<EvaluationResult>>>
}

impl<'cx, 'gcx, 'tcx> SelectionContext<'cx, 'gcx, 'tcx> {
    pub fn new(infcx: &'cx InferCtxt<'cx, 'gcx, 'tcx>) -> SelectionContext<'cx, 'gcx, 'tcx> {
        SelectionContext {
            infcx,
            freshener: infcx.freshener(),
            intercrate: None,
            intercrate_ambiguity_causes: None,
            allow_negative_impls: false,
            query_mode: TraitQueryMode::Standard,
        }
    }

    pub fn intercrate(infcx: &'cx InferCtxt<'cx, 'gcx, 'tcx>,
                      mode: IntercrateMode) -> SelectionContext<'cx, 'gcx, 'tcx> {
        debug!("intercrate({:?})", mode);
        SelectionContext {
            infcx,
            freshener: infcx.freshener(),
            intercrate: Some(mode),
            intercrate_ambiguity_causes: None,
            allow_negative_impls: false,
            query_mode: TraitQueryMode::Standard,
        }
    }

    pub fn with_negative(infcx: &'cx InferCtxt<'cx, 'gcx, 'tcx>,
                         allow_negative_impls: bool) -> SelectionContext<'cx, 'gcx, 'tcx> {
        debug!("with_negative({:?})", allow_negative_impls);
        SelectionContext {
            infcx,
            freshener: infcx.freshener(),
            intercrate: None,
            intercrate_ambiguity_causes: None,
            allow_negative_impls,
            query_mode: TraitQueryMode::Standard,
        }
    }

    pub fn with_query_mode(infcx: &'cx InferCtxt<'cx, 'gcx, 'tcx>,
                           query_mode: TraitQueryMode) -> SelectionContext<'cx, 'gcx, 'tcx> {
        debug!("with_query_mode({:?})", query_mode);
        SelectionContext {
            infcx,
            freshener: infcx.freshener(),
            intercrate: None,
            intercrate_ambiguity_causes: None,
            allow_negative_impls: false,
            query_mode,
        }
    }

    /// Enables tracking of intercrate ambiguity causes. These are
    /// used in coherence to give improved diagnostics. We don't do
    /// this until we detect a coherence error because it can lead to
    /// false overflow results (#47139) and because it costs
    /// computation time.
    pub fn enable_tracking_intercrate_ambiguity_causes(&mut self) {
        assert!(self.intercrate.is_some());
        assert!(self.intercrate_ambiguity_causes.is_none());
        self.intercrate_ambiguity_causes = Some(vec![]);
        debug!("selcx: enable_tracking_intercrate_ambiguity_causes");
    }

    /// Gets the intercrate ambiguity causes collected since tracking
    /// was enabled and disables tracking at the same time. If
    /// tracking is not enabled, just returns an empty vector.
    pub fn take_intercrate_ambiguity_causes(&mut self) -> Vec<IntercrateAmbiguityCause> {
        assert!(self.intercrate.is_some());
        self.intercrate_ambiguity_causes.take().unwrap_or(vec![])
    }

    pub fn infcx(&self) -> &'cx InferCtxt<'cx, 'gcx, 'tcx> {
        self.infcx
    }

    pub fn tcx(&self) -> TyCtxt<'cx, 'gcx, 'tcx> {
        self.infcx.tcx
    }

    pub fn closure_typer(&self) -> &'cx InferCtxt<'cx, 'gcx, 'tcx> {
        self.infcx
    }

    /// Wraps the inference context's in_snapshot s.t. snapshot handling is only from the selection
    /// context's self.
    fn in_snapshot<R, F>(&mut self, f: F) -> R
        where F: FnOnce(&mut Self, &infer::CombinedSnapshot<'cx, 'tcx>) -> R
    {
        self.infcx.in_snapshot(|snapshot| f(self, snapshot))
    }

    /// Wraps a probe s.t. obligations collected during it are ignored and old obligations are
    /// retained.
    fn probe<R, F>(&mut self, f: F) -> R
        where F: FnOnce(&mut Self, &infer::CombinedSnapshot<'cx, 'tcx>) -> R
    {
        self.infcx.probe(|snapshot| f(self, snapshot))
    }

    /// Wraps a commit_if_ok s.t. obligations collected during it are not returned in selection if
    /// the transaction fails and s.t. old obligations are retained.
    fn commit_if_ok<T, E, F>(&mut self, f: F) -> Result<T, E> where
        F: FnOnce(&mut Self, &infer::CombinedSnapshot) -> Result<T, E>
    {
        self.infcx.commit_if_ok(|snapshot| f(self, snapshot))
    }


    ///////////////////////////////////////////////////////////////////////////
    // Selection
    //
    // The selection phase tries to identify *how* an obligation will
    // be resolved. For example, it will identify which impl or
    // parameter bound is to be used. The process can be inconclusive
    // if the self type in the obligation is not fully inferred. Selection
    // can result in an error in one of two ways:
    //
    // 1. If no applicable impl or parameter bound can be found.
    // 2. If the output type parameters in the obligation do not match
    //    those specified by the impl/bound. For example, if the obligation
    //    is `Vec<Foo>:Iterable<Bar>`, but the impl specifies
    //    `impl<T> Iterable<T> for Vec<T>`, than an error would result.

    /// Attempts to satisfy the obligation. If successful, this will affect the surrounding
    /// type environment by performing unification.
    pub fn select(&mut self, obligation: &TraitObligation<'tcx>)
                  -> SelectionResult<'tcx, Selection<'tcx>> {
        debug!("select({:?})", obligation);
        assert!(!obligation.predicate.has_escaping_regions());

        let stack = self.push_stack(TraitObligationStackList::empty(), obligation);

        let candidate = match self.candidate_from_obligation(&stack) {
            Err(SelectionError::Overflow) => {
                // In standard mode, overflow must have been caught and reported
                // earlier.
                assert!(self.query_mode == TraitQueryMode::Canonical);
                return Err(SelectionError::Overflow);
            },
            Err(e) => { return Err(e); },
            Ok(None) => { return Ok(None); },
            Ok(Some(candidate)) => candidate
        };

        match self.confirm_candidate(obligation, candidate) {
            Err(SelectionError::Overflow) => {
                assert!(self.query_mode == TraitQueryMode::Canonical);
                return Err(SelectionError::Overflow);
            },
            Err(e) => Err(e),
            Ok(candidate) => Ok(Some(candidate))
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // EVALUATION
    //
    // Tests whether an obligation can be selected or whether an impl
    // can be applied to particular types. It skips the "confirmation"
    // step and hence completely ignores output type parameters.
    //
    // The result is "true" if the obligation *may* hold and "false" if
    // we can be sure it does not.

    /// Evaluates whether the obligation `obligation` can be satisfied (by any means).
    pub fn predicate_may_hold_fatal(&mut self,
                                    obligation: &PredicateObligation<'tcx>)
                                    -> bool
    {
        debug!("predicate_may_hold_fatal({:?})",
               obligation);

        // This fatal query is a stopgap that should only be used in standard mode,
        // where we do not expect overflow to be propagated.
        assert!(self.query_mode == TraitQueryMode::Standard);

        self.evaluate_obligation_recursively(obligation)
            .expect("Overflow should be caught earlier in standard query mode")
            .may_apply()
    }

    /// Evaluates whether the obligation `obligation` can be satisfied and returns
    /// an `EvaluationResult`.
    pub fn evaluate_obligation_recursively(&mut self,
                                           obligation: &PredicateObligation<'tcx>)
                                           -> Result<EvaluationResult, OverflowError>
    {
        self.probe(|this, _| {
            this.evaluate_predicate_recursively(TraitObligationStackList::empty(), obligation)
        })
    }

    /// Evaluates the predicates in `predicates` recursively. Note that
    /// this applies projections in the predicates, and therefore
    /// is run within an inference probe.
    fn evaluate_predicates_recursively<'a,'o,I>(&mut self,
                                                stack: TraitObligationStackList<'o, 'tcx>,
                                                predicates: I)
                                                -> Result<EvaluationResult, OverflowError>
        where I : IntoIterator<Item=&'a PredicateObligation<'tcx>>, 'tcx:'a
    {
        let mut result = EvaluatedToOk;
        for obligation in predicates {
            let eval = self.evaluate_predicate_recursively(stack, obligation)?;
            debug!("evaluate_predicate_recursively({:?}) = {:?}",
                   obligation, eval);
            if let EvaluatedToErr = eval {
                // fast-path - EvaluatedToErr is the top of the lattice,
                // so we don't need to look on the other predicates.
                return Ok(EvaluatedToErr);
            } else {
                result = cmp::max(result, eval);
            }
        }
        Ok(result)
    }

    fn evaluate_predicate_recursively<'o>(&mut self,
                                          previous_stack: TraitObligationStackList<'o, 'tcx>,
                                          obligation: &PredicateObligation<'tcx>)
                                           -> Result<EvaluationResult, OverflowError>
    {
        debug!("evaluate_predicate_recursively({:?})",
               obligation);

        match obligation.predicate {
            ty::Predicate::Trait(ref t) => {
                assert!(!t.has_escaping_regions());
                let obligation = obligation.with(t.clone());
                self.evaluate_trait_predicate_recursively(previous_stack, obligation)
            }

            ty::Predicate::Subtype(ref p) => {
                // does this code ever run?
                match self.infcx.subtype_predicate(&obligation.cause, obligation.param_env, p) {
                    Some(Ok(InferOk { obligations, .. })) => {
                        self.evaluate_predicates_recursively(previous_stack, &obligations)
                    },
                    Some(Err(_)) => Ok(EvaluatedToErr),
                    None => Ok(EvaluatedToAmbig),
                }
            }

            ty::Predicate::WellFormed(ty) => {
                match ty::wf::obligations(self.infcx,
                                          obligation.param_env,
                                          obligation.cause.body_id,
                                          ty, obligation.cause.span) {
                    Some(obligations) =>
                        self.evaluate_predicates_recursively(previous_stack, obligations.iter()),
                    None =>
                        Ok(EvaluatedToAmbig),
                }
            }

            ty::Predicate::TypeOutlives(..) | ty::Predicate::RegionOutlives(..) => {
                // we do not consider region relationships when
                // evaluating trait matches
                Ok(EvaluatedToOk)
            }

            ty::Predicate::ObjectSafe(trait_def_id) => {
                if self.tcx().is_object_safe(trait_def_id) {
                    Ok(EvaluatedToOk)
                } else {
                    Ok(EvaluatedToErr)
                }
            }

            ty::Predicate::Projection(ref data) => {
                let project_obligation = obligation.with(data.clone());
                match project::poly_project_and_unify_type(self, &project_obligation) {
                    Ok(Some(subobligations)) => {
                        let result = self.evaluate_predicates_recursively(previous_stack,
                                                                          subobligations.iter());
                        if let Some(key) =
                            ProjectionCacheKey::from_poly_projection_predicate(self, data)
                        {
                            self.infcx.projection_cache.borrow_mut().complete(key);
                        }
                        result
                    }
                    Ok(None) => {
                        Ok(EvaluatedToAmbig)
                    }
                    Err(_) => {
                        Ok(EvaluatedToErr)
                    }
                }
            }

            ty::Predicate::ClosureKind(closure_def_id, closure_substs, kind) => {
                match self.infcx.closure_kind(closure_def_id, closure_substs) {
                    Some(closure_kind) => {
                        if closure_kind.extends(kind) {
                            Ok(EvaluatedToOk)
                        } else {
                            Ok(EvaluatedToErr)
                        }
                    }
                    None => {
                        Ok(EvaluatedToAmbig)
                    }
                }
            }

            ty::Predicate::ConstEvaluatable(def_id, substs) => {
                let tcx = self.tcx();
                match tcx.lift_to_global(&(obligation.param_env, substs)) {
                    Some((param_env, substs)) => {
                        let instance = ty::Instance::resolve(
                            tcx.global_tcx(),
                            param_env,
                            def_id,
                            substs,
                        );
                        if let Some(instance) = instance {
                            let cid = GlobalId {
                                instance,
                                promoted: None
                            };
                            match self.tcx().const_eval(param_env.and(cid)) {
                                Ok(_) => Ok(EvaluatedToOk),
                                Err(_) => Ok(EvaluatedToErr)
                            }
                        } else {
                            Ok(EvaluatedToErr)
                        }
                    }
                    None => {
                        // Inference variables still left in param_env or substs.
                        Ok(EvaluatedToAmbig)
                    }
                }
            }
        }
    }

    fn evaluate_trait_predicate_recursively<'o>(&mut self,
                                                previous_stack: TraitObligationStackList<'o, 'tcx>,
                                                mut obligation: TraitObligation<'tcx>)
                                                -> Result<EvaluationResult, OverflowError>
    {
        debug!("evaluate_trait_predicate_recursively({:?})", obligation);

        if self.intercrate.is_none() && obligation.is_global()
            && obligation.param_env.caller_bounds.iter().all(|bound| bound.needs_subst()) {
            // If a param env has no global bounds, global obligations do not
            // depend on its particular value in order to work, so we can clear
            // out the param env and get better caching.
            debug!("evaluate_trait_predicate_recursively({:?}) - in global", obligation);
            obligation.param_env = obligation.param_env.without_caller_bounds();
        }

        let stack = self.push_stack(previous_stack, &obligation);
        let fresh_trait_ref = stack.fresh_trait_ref;
        if let Some(result) = self.check_evaluation_cache(obligation.param_env, fresh_trait_ref) {
            debug!("CACHE HIT: EVAL({:?})={:?}",
                   fresh_trait_ref,
                   result);
            return Ok(result);
        }

        let (result, dep_node) = self.in_task(|this| this.evaluate_stack(&stack));
        let result = result?;

        debug!("CACHE MISS: EVAL({:?})={:?}",
               fresh_trait_ref,
               result);
        self.insert_evaluation_cache(obligation.param_env, fresh_trait_ref, dep_node, result);

        Ok(result)
    }

    fn evaluate_stack<'o>(&mut self,
                          stack: &TraitObligationStack<'o, 'tcx>)
                          -> Result<EvaluationResult, OverflowError>
    {
        // In intercrate mode, whenever any of the types are unbound,
        // there can always be an impl. Even if there are no impls in
        // this crate, perhaps the type would be unified with
        // something from another crate that does provide an impl.
        //
        // In intra mode, we must still be conservative. The reason is
        // that we want to avoid cycles. Imagine an impl like:
        //
        //     impl<T:Eq> Eq for Vec<T>
        //
        // and a trait reference like `$0 : Eq` where `$0` is an
        // unbound variable. When we evaluate this trait-reference, we
        // will unify `$0` with `Vec<$1>` (for some fresh variable
        // `$1`), on the condition that `$1 : Eq`. We will then wind
        // up with many candidates (since that are other `Eq` impls
        // that apply) and try to winnow things down. This results in
        // a recursive evaluation that `$1 : Eq` -- as you can
        // imagine, this is just where we started. To avoid that, we
        // check for unbound variables and return an ambiguous (hence possible)
        // match if we've seen this trait before.
        //
        // This suffices to allow chains like `FnMut` implemented in
        // terms of `Fn` etc, but we could probably make this more
        // precise still.
        let unbound_input_types =
            stack.fresh_trait_ref.skip_binder().input_types().any(|ty| ty.is_fresh());
        // this check was an imperfect workaround for a bug n the old
        // intercrate mode, it should be removed when that goes away.
        if unbound_input_types &&
            self.intercrate == Some(IntercrateMode::Issue43355)
        {
            debug!("evaluate_stack({:?}) --> unbound argument, intercrate -->  ambiguous",
                   stack.fresh_trait_ref);
            // Heuristics: show the diagnostics when there are no candidates in crate.
            if self.intercrate_ambiguity_causes.is_some() {
                debug!("evaluate_stack: intercrate_ambiguity_causes is some");
                if let Ok(candidate_set) = self.assemble_candidates(stack) {
                    if !candidate_set.ambiguous && candidate_set.vec.is_empty() {
                        let trait_ref = stack.obligation.predicate.skip_binder().trait_ref;
                        let self_ty = trait_ref.self_ty();
                        let cause = IntercrateAmbiguityCause::DownstreamCrate {
                            trait_desc: trait_ref.to_string(),
                            self_desc: if self_ty.has_concrete_skeleton() {
                                Some(self_ty.to_string())
                            } else {
                                None
                            },
                        };
                        debug!("evaluate_stack: pushing cause = {:?}", cause);
                        self.intercrate_ambiguity_causes.as_mut().unwrap().push(cause);
                    }
                }
            }
            return Ok(EvaluatedToAmbig);
        }
        if unbound_input_types &&
              stack.iter().skip(1).any(
                  |prev| stack.obligation.param_env == prev.obligation.param_env &&
                      self.match_fresh_trait_refs(&stack.fresh_trait_ref,
                                                  &prev.fresh_trait_ref))
        {
            debug!("evaluate_stack({:?}) --> unbound argument, recursive --> giving up",
                   stack.fresh_trait_ref);
            return Ok(EvaluatedToUnknown);
        }

        // If there is any previous entry on the stack that precisely
        // matches this obligation, then we can assume that the
        // obligation is satisfied for now (still all other conditions
        // must be met of course). One obvious case this comes up is
        // marker traits like `Send`. Think of a linked list:
        //
        //    struct List<T> { data: T, next: Option<Box<List<T>>> {
        //
        // `Box<List<T>>` will be `Send` if `T` is `Send` and
        // `Option<Box<List<T>>>` is `Send`, and in turn
        // `Option<Box<List<T>>>` is `Send` if `Box<List<T>>` is
        // `Send`.
        //
        // Note that we do this comparison using the `fresh_trait_ref`
        // fields. Because these have all been skolemized using
        // `self.freshener`, we can be sure that (a) this will not
        // affect the inferencer state and (b) that if we see two
        // skolemized types with the same index, they refer to the
        // same unbound type variable.
        if let Some(rec_index) =
            stack.iter()
            .skip(1) // skip top-most frame
            .position(|prev| stack.obligation.param_env == prev.obligation.param_env &&
                      stack.fresh_trait_ref == prev.fresh_trait_ref)
        {
            debug!("evaluate_stack({:?}) --> recursive",
                   stack.fresh_trait_ref);
            let cycle = stack.iter().skip(1).take(rec_index+1);
            let cycle = cycle.map(|stack| ty::Predicate::Trait(stack.obligation.predicate));
            if self.coinductive_match(cycle) {
                debug!("evaluate_stack({:?}) --> recursive, coinductive",
                       stack.fresh_trait_ref);
                return Ok(EvaluatedToOk);
            } else {
                debug!("evaluate_stack({:?}) --> recursive, inductive",
                       stack.fresh_trait_ref);
                return Ok(EvaluatedToRecur);
            }
        }

        match self.candidate_from_obligation(stack) {
            Ok(Some(c)) => self.evaluate_candidate(stack, &c),
            Ok(None) => Ok(EvaluatedToAmbig),
            Err(Overflow) => Err(OverflowError),
            Err(..) => Ok(EvaluatedToErr)
        }
    }

    /// For defaulted traits, we use a co-inductive strategy to solve, so
    /// that recursion is ok. This routine returns true if the top of the
    /// stack (`cycle[0]`):
    ///
    /// - is a defaulted trait, and
    /// - it also appears in the backtrace at some position `X`; and,
    /// - all the predicates at positions `X..` between `X` an the top are
    ///   also defaulted traits.
    pub fn coinductive_match<I>(&mut self, cycle: I) -> bool
        where I: Iterator<Item=ty::Predicate<'tcx>>
    {
        let mut cycle = cycle;
        cycle.all(|predicate| self.coinductive_predicate(predicate))
    }

    fn coinductive_predicate(&self, predicate: ty::Predicate<'tcx>) -> bool {
        let result = match predicate {
            ty::Predicate::Trait(ref data) => {
                self.tcx().trait_is_auto(data.def_id())
            }
            _ => {
                false
            }
        };
        debug!("coinductive_predicate({:?}) = {:?}", predicate, result);
        result
    }

    /// Further evaluate `candidate` to decide whether all type parameters match and whether nested
    /// obligations are met. Returns true if `candidate` remains viable after this further
    /// scrutiny.
    fn evaluate_candidate<'o>(&mut self,
                              stack: &TraitObligationStack<'o, 'tcx>,
                              candidate: &SelectionCandidate<'tcx>)
                              -> Result<EvaluationResult, OverflowError>
    {
        debug!("evaluate_candidate: depth={} candidate={:?}",
               stack.obligation.recursion_depth, candidate);
        let result = self.probe(|this, _| {
            let candidate = (*candidate).clone();
            match this.confirm_candidate(stack.obligation, candidate) {
                Ok(selection) => {
                    this.evaluate_predicates_recursively(
                        stack.list(),
                        selection.nested_obligations().iter())
                }
                Err(..) => Ok(EvaluatedToErr)
            }
        })?;
        debug!("evaluate_candidate: depth={} result={:?}",
               stack.obligation.recursion_depth, result);
        Ok(result)
    }

    fn check_evaluation_cache(&self,
                              param_env: ty::ParamEnv<'tcx>,
                              trait_ref: ty::PolyTraitRef<'tcx>)
                              -> Option<EvaluationResult>
    {
        let tcx = self.tcx();
        if self.can_use_global_caches(param_env) {
            let cache = tcx.evaluation_cache.hashmap.borrow();
            if let Some(cached) = cache.get(&trait_ref) {
                return Some(cached.get(tcx));
            }
        }
        self.infcx.evaluation_cache.hashmap
                                   .borrow()
                                   .get(&trait_ref)
                                   .map(|v| v.get(tcx))
    }

    fn insert_evaluation_cache(&mut self,
                               param_env: ty::ParamEnv<'tcx>,
                               trait_ref: ty::PolyTraitRef<'tcx>,
                               dep_node: DepNodeIndex,
                               result: EvaluationResult)
    {
        // Avoid caching results that depend on more than just the trait-ref
        // - the stack can create recursion.
        if result.is_stack_dependent() {
            return;
        }

        if self.can_use_global_caches(param_env) {
            if let Some(trait_ref) = self.tcx().lift_to_global(&trait_ref) {
                debug!(
                    "insert_evaluation_cache(trait_ref={:?}, candidate={:?}) global",
                    trait_ref,
                    result,
                );
                // This may overwrite the cache with the same value
                // FIXME: Due to #50507 this overwrites the different values
                // This should be changed to use HashMapExt::insert_same
                // when that is fixed
                self.tcx().evaluation_cache
                          .hashmap.borrow_mut()
                          .insert(trait_ref, WithDepNode::new(dep_node, result));
                return;
            }
        }

        debug!(
            "insert_evaluation_cache(trait_ref={:?}, candidate={:?})",
            trait_ref,
            result,
        );
        self.infcx.evaluation_cache.hashmap
                                   .borrow_mut()
                                   .insert(trait_ref, WithDepNode::new(dep_node, result));
    }

    ///////////////////////////////////////////////////////////////////////////
    // CANDIDATE ASSEMBLY
    //
    // The selection process begins by examining all in-scope impls,
    // caller obligations, and so forth and assembling a list of
    // candidates. See [rustc guide] for more details.
    //
    // [rustc guide]:
    // https://rust-lang-nursery.github.io/rustc-guide/trait-resolution.html#candidate-assembly

    fn candidate_from_obligation<'o>(&mut self,
                                     stack: &TraitObligationStack<'o, 'tcx>)
                                     -> SelectionResult<'tcx, SelectionCandidate<'tcx>>
    {
        // Watch out for overflow. This intentionally bypasses (and does
        // not update) the cache.
        let recursion_limit = *self.infcx.tcx.sess.recursion_limit.get();
        if stack.obligation.recursion_depth >= recursion_limit {
            match self.query_mode {
                TraitQueryMode::Standard => {
                    self.infcx().report_overflow_error(&stack.obligation, true);
                },
                TraitQueryMode::Canonical => {
                    return Err(Overflow);
                },
            }
        }

        // Check the cache. Note that we skolemize the trait-ref
        // separately rather than using `stack.fresh_trait_ref` -- this
        // is because we want the unbound variables to be replaced
        // with fresh skolemized types starting from index 0.
        let cache_fresh_trait_pred =
            self.infcx.freshen(stack.obligation.predicate.clone());
        debug!("candidate_from_obligation(cache_fresh_trait_pred={:?}, obligation={:?})",
               cache_fresh_trait_pred,
               stack);
        assert!(!stack.obligation.predicate.has_escaping_regions());

        if let Some(c) = self.check_candidate_cache(stack.obligation.param_env,
                                                    &cache_fresh_trait_pred) {
            debug!("CACHE HIT: SELECT({:?})={:?}",
                   cache_fresh_trait_pred,
                   c);
            return c;
        }

        // If no match, compute result and insert into cache.
        let (candidate, dep_node) = self.in_task(|this| {
            this.candidate_from_obligation_no_cache(stack)
        });

        debug!("CACHE MISS: SELECT({:?})={:?}",
               cache_fresh_trait_pred, candidate);
        self.insert_candidate_cache(stack.obligation.param_env,
                                    cache_fresh_trait_pred,
                                    dep_node,
                                    candidate.clone());
        candidate
    }

    fn in_task<OP, R>(&mut self, op: OP) -> (R, DepNodeIndex)
        where OP: FnOnce(&mut Self) -> R
    {
        let (result, dep_node) = self.tcx().dep_graph.with_anon_task(DepKind::TraitSelect, || {
            op(self)
        });
        self.tcx().dep_graph.read_index(dep_node);
        (result, dep_node)
    }

    // Treat negative impls as unimplemented
    fn filter_negative_impls(&self, candidate: SelectionCandidate<'tcx>)
                             -> SelectionResult<'tcx, SelectionCandidate<'tcx>> {
        if let ImplCandidate(def_id) = candidate {
            if !self.allow_negative_impls &&
                self.tcx().impl_polarity(def_id) == hir::ImplPolarity::Negative {
                return Err(Unimplemented)
            }
        }
        Ok(Some(candidate))
    }

    fn candidate_from_obligation_no_cache<'o>(&mut self,
                                              stack: &TraitObligationStack<'o, 'tcx>)
                                              -> SelectionResult<'tcx, SelectionCandidate<'tcx>>
    {
        if stack.obligation.predicate.references_error() {
            // If we encounter a `TyError`, we generally prefer the
            // most "optimistic" result in response -- that is, the
            // one least likely to report downstream errors. But
            // because this routine is shared by coherence and by
            // trait selection, there isn't an obvious "right" choice
            // here in that respect, so we opt to just return
            // ambiguity and let the upstream clients sort it out.
            return Ok(None);
        }

        match self.is_knowable(stack) {
            None => {}
            Some(conflict) => {
                debug!("coherence stage: not knowable");
                if self.intercrate_ambiguity_causes.is_some() {
                    debug!("evaluate_stack: intercrate_ambiguity_causes is some");
                    // Heuristics: show the diagnostics when there are no candidates in crate.
                    if let Ok(candidate_set) = self.assemble_candidates(stack) {
                        let no_candidates_apply =
                            candidate_set
                            .vec
                            .iter()
                            .map(|c| self.evaluate_candidate(stack, &c))
                            .collect::<Result<Vec<_>, OverflowError>>()?
                            .iter()
                            .all(|r| !r.may_apply());
                        if !candidate_set.ambiguous && no_candidates_apply {
                            let trait_ref = stack.obligation.predicate.skip_binder().trait_ref;
                            let self_ty = trait_ref.self_ty();
                            let trait_desc = trait_ref.to_string();
                            let self_desc = if self_ty.has_concrete_skeleton() {
                                Some(self_ty.to_string())
                            } else {
                                None
                            };
                            let cause = if let Conflict::Upstream = conflict {
                                IntercrateAmbiguityCause::UpstreamCrateUpdate {
                                    trait_desc,
                                    self_desc,
                                }
                            } else {
                                IntercrateAmbiguityCause::DownstreamCrate { trait_desc, self_desc }
                            };
                            debug!("evaluate_stack: pushing cause = {:?}", cause);
                            self.intercrate_ambiguity_causes.as_mut().unwrap().push(cause);
                        }
                    }
                }
                return Ok(None);
            }
        }

        let candidate_set = self.assemble_candidates(stack)?;

        if candidate_set.ambiguous {
            debug!("candidate set contains ambig");
            return Ok(None);
        }

        let mut candidates = candidate_set.vec;

        debug!("assembled {} candidates for {:?}: {:?}",
               candidates.len(),
               stack,
               candidates);

        // At this point, we know that each of the entries in the
        // candidate set is *individually* applicable. Now we have to
        // figure out if they contain mutual incompatibilities. This
        // frequently arises if we have an unconstrained input type --
        // for example, we are looking for $0:Eq where $0 is some
        // unconstrained type variable. In that case, we'll get a
        // candidate which assumes $0 == int, one that assumes $0 ==
        // usize, etc. This spells an ambiguity.

        // If there is more than one candidate, first winnow them down
        // by considering extra conditions (nested obligations and so
        // forth). We don't winnow if there is exactly one
        // candidate. This is a relatively minor distinction but it
        // can lead to better inference and error-reporting. An
        // example would be if there was an impl:
        //
        //     impl<T:Clone> Vec<T> { fn push_clone(...) { ... } }
        //
        // and we were to see some code `foo.push_clone()` where `boo`
        // is a `Vec<Bar>` and `Bar` does not implement `Clone`.  If
        // we were to winnow, we'd wind up with zero candidates.
        // Instead, we select the right impl now but report `Bar does
        // not implement Clone`.
        if candidates.len() == 1 {
            return self.filter_negative_impls(candidates.pop().unwrap());
        }

        // Winnow, but record the exact outcome of evaluation, which
        // is needed for specialization. Propagate overflow if it occurs.
        let candidates: Result<Vec<Option<EvaluatedCandidate>>, _> = candidates
            .into_iter()
            .map(|c| match self.evaluate_candidate(stack, &c) {
                Ok(eval) if eval.may_apply() => Ok(Some(EvaluatedCandidate {
                    candidate: c,
                    evaluation: eval,
                })),
                Ok(_) => Ok(None),
                Err(OverflowError) => Err(Overflow),
            })
            .collect();

        let mut candidates: Vec<EvaluatedCandidate> =
            candidates?.into_iter().filter_map(|c| c).collect();

        // If there are STILL multiple candidate, we can further
        // reduce the list by dropping duplicates -- including
        // resolving specializations.
        if candidates.len() > 1 {
            let mut i = 0;
            while i < candidates.len() {
                let is_dup =
                    (0..candidates.len())
                    .filter(|&j| i != j)
                    .any(|j| self.candidate_should_be_dropped_in_favor_of(&candidates[i],
                                                                          &candidates[j]));
                if is_dup {
                    debug!("Dropping candidate #{}/{}: {:?}",
                           i, candidates.len(), candidates[i]);
                    candidates.swap_remove(i);
                } else {
                    debug!("Retaining candidate #{}/{}: {:?}",
                           i, candidates.len(), candidates[i]);
                    i += 1;

                    // If there are *STILL* multiple candidates, give up
                    // and report ambiguity.
                    if i > 1 {
                        debug!("multiple matches, ambig");
                        return Ok(None);
                    }
                }
            }
        }

        // If there are *NO* candidates, then there are no impls --
        // that we know of, anyway. Note that in the case where there
        // are unbound type variables within the obligation, it might
        // be the case that you could still satisfy the obligation
        // from another crate by instantiating the type variables with
        // a type from another crate that does have an impl. This case
        // is checked for in `evaluate_stack` (and hence users
        // who might care about this case, like coherence, should use
        // that function).
        if candidates.is_empty() {
            return Err(Unimplemented);
        }

        // Just one candidate left.
        self.filter_negative_impls(candidates.pop().unwrap().candidate)
    }

    fn is_knowable<'o>(&mut self,
                       stack: &TraitObligationStack<'o, 'tcx>)
                       -> Option<Conflict>
    {
        debug!("is_knowable(intercrate={:?})", self.intercrate);

        if !self.intercrate.is_some() {
            return None;
        }

        let obligation = &stack.obligation;
        let predicate = self.infcx().resolve_type_vars_if_possible(&obligation.predicate);

        // ok to skip binder because of the nature of the
        // trait-ref-is-knowable check, which does not care about
        // bound regions
        let trait_ref = predicate.skip_binder().trait_ref;

        let result = coherence::trait_ref_is_knowable(self.tcx(), trait_ref);
        if let (Some(Conflict::Downstream { used_to_be_broken: true }),
                Some(IntercrateMode::Issue43355)) = (result, self.intercrate) {
            debug!("is_knowable: IGNORING conflict to be bug-compatible with #43355");
            None
        } else {
            result
        }
    }

    /// Returns true if the global caches can be used.
    /// Do note that if the type itself is not in the
    /// global tcx, the local caches will be used.
    fn can_use_global_caches(&self, param_env: ty::ParamEnv<'tcx>) -> bool {
        // If there are any where-clauses in scope, then we always use
        // a cache local to this particular scope. Otherwise, we
        // switch to a global cache. We used to try and draw
        // finer-grained distinctions, but that led to a serious of
        // annoying and weird bugs like #22019 and #18290. This simple
        // rule seems to be pretty clearly safe and also still retains
        // a very high hit rate (~95% when compiling rustc).
        if !param_env.caller_bounds.is_empty() {
            return false;
        }

        // Avoid using the master cache during coherence and just rely
        // on the local cache. This effectively disables caching
        // during coherence. It is really just a simplification to
        // avoid us having to fear that coherence results "pollute"
        // the master cache. Since coherence executes pretty quickly,
        // it's not worth going to more trouble to increase the
        // hit-rate I don't think.
        if self.intercrate.is_some() {
            return false;
        }

        // Otherwise, we can use the global cache.
        true
    }

    fn check_candidate_cache(&mut self,
                             param_env: ty::ParamEnv<'tcx>,
                             cache_fresh_trait_pred: &ty::PolyTraitPredicate<'tcx>)
                             -> Option<SelectionResult<'tcx, SelectionCandidate<'tcx>>>
    {
        let tcx = self.tcx();
        let trait_ref = &cache_fresh_trait_pred.skip_binder().trait_ref;
        if self.can_use_global_caches(param_env) {
            let cache = tcx.selection_cache.hashmap.borrow();
            if let Some(cached) = cache.get(&trait_ref) {
                return Some(cached.get(tcx));
            }
        }
        self.infcx.selection_cache.hashmap
                                  .borrow()
                                  .get(trait_ref)
                                  .map(|v| v.get(tcx))
    }

    fn insert_candidate_cache(&mut self,
                              param_env: ty::ParamEnv<'tcx>,
                              cache_fresh_trait_pred: ty::PolyTraitPredicate<'tcx>,
                              dep_node: DepNodeIndex,
                              candidate: SelectionResult<'tcx, SelectionCandidate<'tcx>>)
    {
        let tcx = self.tcx();
        let trait_ref = cache_fresh_trait_pred.skip_binder().trait_ref;
        if self.can_use_global_caches(param_env) {
            if let Some(trait_ref) = tcx.lift_to_global(&trait_ref) {
                if let Some(candidate) = tcx.lift_to_global(&candidate) {
                    debug!(
                        "insert_candidate_cache(trait_ref={:?}, candidate={:?}) global",
                        trait_ref,
                        candidate,
                    );
                    // This may overwrite the cache with the same value
                    tcx.selection_cache
                       .hashmap.borrow_mut()
                       .insert(trait_ref, WithDepNode::new(dep_node, candidate));
                    return;
                }
            }
        }

        debug!(
            "insert_candidate_cache(trait_ref={:?}, candidate={:?}) local",
            trait_ref,
            candidate,
        );
        self.infcx.selection_cache.hashmap
                                  .borrow_mut()
                                  .insert(trait_ref, WithDepNode::new(dep_node, candidate));
    }

    fn assemble_candidates<'o>(&mut self,
                               stack: &TraitObligationStack<'o, 'tcx>)
                               -> Result<SelectionCandidateSet<'tcx>, SelectionError<'tcx>>
    {
        let TraitObligationStack { obligation, .. } = *stack;
        let ref obligation = Obligation {
            param_env: obligation.param_env,
            cause: obligation.cause.clone(),
            recursion_depth: obligation.recursion_depth,
            predicate: self.infcx().resolve_type_vars_if_possible(&obligation.predicate)
        };

        if obligation.predicate.skip_binder().self_ty().is_ty_var() {
            // Self is a type variable (e.g. `_: AsRef<str>`).
            //
            // This is somewhat problematic, as the current scheme can't really
            // handle it turning to be a projection. This does end up as truly
            // ambiguous in most cases anyway.
            //
            // Take the fast path out - this also improves
            // performance by preventing assemble_candidates_from_impls from
            // matching every impl for this trait.
            return Ok(SelectionCandidateSet { vec: vec![], ambiguous: true });
        }

        let mut candidates = SelectionCandidateSet {
            vec: Vec::new(),
            ambiguous: false
        };

        // Other bounds. Consider both in-scope bounds from fn decl
        // and applicable impls. There is a certain set of precedence rules here.

        let def_id = obligation.predicate.def_id();
        let lang_items = self.tcx().lang_items();
        if lang_items.copy_trait() == Some(def_id) {
            debug!("obligation self ty is {:?}",
                   obligation.predicate.skip_binder().self_ty());

            // User-defined copy impls are permitted, but only for
            // structs and enums.
            self.assemble_candidates_from_impls(obligation, &mut candidates)?;

            // For other types, we'll use the builtin rules.
            let copy_conditions = self.copy_clone_conditions(obligation);
            self.assemble_builtin_bound_candidates(copy_conditions, &mut candidates)?;
        } else if lang_items.sized_trait() == Some(def_id) {
            // Sized is never implementable by end-users, it is
            // always automatically computed.
            let sized_conditions = self.sized_conditions(obligation);
            self.assemble_builtin_bound_candidates(sized_conditions,
                                                   &mut candidates)?;
        } else if lang_items.unsize_trait() == Some(def_id) {
            self.assemble_candidates_for_unsizing(obligation, &mut candidates);
        } else {
            if lang_items.clone_trait() == Some(def_id) {
                // Same builtin conditions as `Copy`, i.e. every type which has builtin support
                // for `Copy` also has builtin support for `Clone`, + tuples and arrays of `Clone`
                // types have builtin support for `Clone`.
                let clone_conditions = self.copy_clone_conditions(obligation);
                self.assemble_builtin_bound_candidates(clone_conditions, &mut candidates)?;
            }

            self.assemble_generator_candidates(obligation, &mut candidates)?;
            self.assemble_closure_candidates(obligation, &mut candidates)?;
            self.assemble_fn_pointer_candidates(obligation, &mut candidates)?;
            self.assemble_candidates_from_impls(obligation, &mut candidates)?;
            self.assemble_candidates_from_object_ty(obligation, &mut candidates);
        }

        self.assemble_candidates_from_projected_tys(obligation, &mut candidates);
        self.assemble_candidates_from_caller_bounds(stack, &mut candidates)?;
        // Auto implementations have lower priority, so we only
        // consider triggering a default if there is no other impl that can apply.
        if candidates.vec.is_empty() {
            self.assemble_candidates_from_auto_impls(obligation, &mut candidates)?;
        }
        debug!("candidate list size: {}", candidates.vec.len());
        Ok(candidates)
    }

    fn assemble_candidates_from_projected_tys(&mut self,
                                              obligation: &TraitObligation<'tcx>,
                                              candidates: &mut SelectionCandidateSet<'tcx>)
    {
        debug!("assemble_candidates_for_projected_tys({:?})", obligation);

        // before we go into the whole skolemization thing, just
        // quickly check if the self-type is a projection at all.
        match obligation.predicate.skip_binder().trait_ref.self_ty().sty {
            ty::TyProjection(_) | ty::TyAnon(..) => {}
            ty::TyInfer(ty::TyVar(_)) => {
                span_bug!(obligation.cause.span,
                    "Self=_ should have been handled by assemble_candidates");
            }
            _ => return
        }

        let result = self.probe(|this, snapshot| {
            this.match_projection_obligation_against_definition_bounds(obligation,
                                                                       snapshot)
        });

        if result {
            candidates.vec.push(ProjectionCandidate);
        }
    }

    fn match_projection_obligation_against_definition_bounds(
        &mut self,
        obligation: &TraitObligation<'tcx>,
        snapshot: &infer::CombinedSnapshot<'cx, 'tcx>)
        -> bool
    {
        let poly_trait_predicate =
            self.infcx().resolve_type_vars_if_possible(&obligation.predicate);
        let (skol_trait_predicate, skol_map) =
            self.infcx().skolemize_late_bound_regions(&poly_trait_predicate);
        debug!("match_projection_obligation_against_definition_bounds: \
                skol_trait_predicate={:?} skol_map={:?}",
               skol_trait_predicate,
               skol_map);

        let (def_id, substs) = match skol_trait_predicate.trait_ref.self_ty().sty {
            ty::TyProjection(ref data) =>
                (data.trait_ref(self.tcx()).def_id, data.substs),
            ty::TyAnon(def_id, substs) => (def_id, substs),
            _ => {
                span_bug!(
                    obligation.cause.span,
                    "match_projection_obligation_against_definition_bounds() called \
                     but self-ty not a projection: {:?}",
                    skol_trait_predicate.trait_ref.self_ty());
            }
        };
        debug!("match_projection_obligation_against_definition_bounds: \
                def_id={:?}, substs={:?}",
               def_id, substs);

        let predicates_of = self.tcx().predicates_of(def_id);
        let bounds = predicates_of.instantiate(self.tcx(), substs);
        debug!("match_projection_obligation_against_definition_bounds: \
                bounds={:?}",
               bounds);

        let matching_bound =
            util::elaborate_predicates(self.tcx(), bounds.predicates)
            .filter_to_traits()
            .find(
                |bound| self.probe(
                    |this, _| this.match_projection(obligation,
                                                    bound.clone(),
                                                    skol_trait_predicate.trait_ref.clone(),
                                                    &skol_map,
                                                    snapshot)));

        debug!("match_projection_obligation_against_definition_bounds: \
                matching_bound={:?}",
               matching_bound);
        match matching_bound {
            None => false,
            Some(bound) => {
                // Repeat the successful match, if any, this time outside of a probe.
                let result = self.match_projection(obligation,
                                                   bound,
                                                   skol_trait_predicate.trait_ref.clone(),
                                                   &skol_map,
                                                   snapshot);

                self.infcx.pop_skolemized(skol_map, snapshot);

                assert!(result);
                true
            }
        }
    }

    fn match_projection(&mut self,
                        obligation: &TraitObligation<'tcx>,
                        trait_bound: ty::PolyTraitRef<'tcx>,
                        skol_trait_ref: ty::TraitRef<'tcx>,
                        skol_map: &infer::SkolemizationMap<'tcx>,
                        snapshot: &infer::CombinedSnapshot<'cx, 'tcx>)
                        -> bool
    {
        assert!(!skol_trait_ref.has_escaping_regions());
        if let Err(_) = self.infcx.at(&obligation.cause, obligation.param_env)
                                  .sup(ty::Binder::dummy(skol_trait_ref), trait_bound) {
            return false;
        }

        self.infcx.leak_check(false, obligation.cause.span, skol_map, snapshot).is_ok()
    }

    /// Given an obligation like `<SomeTrait for T>`, search the obligations that the caller
    /// supplied to find out whether it is listed among them.
    ///
    /// Never affects inference environment.
    fn assemble_candidates_from_caller_bounds<'o>(&mut self,
                                                  stack: &TraitObligationStack<'o, 'tcx>,
                                                  candidates: &mut SelectionCandidateSet<'tcx>)
                                                  -> Result<(),SelectionError<'tcx>>
    {
        debug!("assemble_candidates_from_caller_bounds({:?})",
               stack.obligation);

        let all_bounds =
            stack.obligation.param_env.caller_bounds
                                      .iter()
                                      .filter_map(|o| o.to_opt_poly_trait_ref());

        // micro-optimization: filter out predicates relating to different
        // traits.
        let matching_bounds =
            all_bounds.filter(|p| p.def_id() == stack.obligation.predicate.def_id());

        // keep only those bounds which may apply, and propagate overflow if it occurs
        let mut param_candidates = vec![];
        for bound in matching_bounds {
            let wc = self.evaluate_where_clause(stack, bound.clone())?;
            if wc.may_apply() {
                param_candidates.push(ParamCandidate(bound));
            }
        }

        candidates.vec.extend(param_candidates);

        Ok(())
    }

    fn evaluate_where_clause<'o>(&mut self,
                                 stack: &TraitObligationStack<'o, 'tcx>,
                                 where_clause_trait_ref: ty::PolyTraitRef<'tcx>)
                                 -> Result<EvaluationResult, OverflowError>
    {
        self.probe(move |this, _| {
            match this.match_where_clause_trait_ref(stack.obligation, where_clause_trait_ref) {
                Ok(obligations) => {
                    this.evaluate_predicates_recursively(stack.list(), obligations.iter())
                }
                Err(()) => Ok(EvaluatedToErr)
            }
        })
    }

    fn assemble_generator_candidates(&mut self,
                                   obligation: &TraitObligation<'tcx>,
                                   candidates: &mut SelectionCandidateSet<'tcx>)
                                   -> Result<(),SelectionError<'tcx>>
    {
        if self.tcx().lang_items().gen_trait() != Some(obligation.predicate.def_id()) {
            return Ok(());
        }

        // ok to skip binder because the substs on generator types never
        // touch bound regions, they just capture the in-scope
        // type/region parameters
        let self_ty = *obligation.self_ty().skip_binder();
        match self_ty.sty {
            ty::TyGenerator(..) => {
                debug!("assemble_generator_candidates: self_ty={:?} obligation={:?}",
                       self_ty,
                       obligation);

                candidates.vec.push(GeneratorCandidate);
                Ok(())
            }
            ty::TyInfer(ty::TyVar(_)) => {
                debug!("assemble_generator_candidates: ambiguous self-type");
                candidates.ambiguous = true;
                return Ok(());
            }
            _ => { return Ok(()); }
        }
    }

    /// Check for the artificial impl that the compiler will create for an obligation like `X :
    /// FnMut<..>` where `X` is a closure type.
    ///
    /// Note: the type parameters on a closure candidate are modeled as *output* type
    /// parameters and hence do not affect whether this trait is a match or not. They will be
    /// unified during the confirmation step.
    fn assemble_closure_candidates(&mut self,
                                   obligation: &TraitObligation<'tcx>,
                                   candidates: &mut SelectionCandidateSet<'tcx>)
                                   -> Result<(),SelectionError<'tcx>>
    {
        let kind = match self.tcx().lang_items().fn_trait_kind(obligation.predicate.def_id()) {
            Some(k) => k,
            None => { return Ok(()); }
        };

        // ok to skip binder because the substs on closure types never
        // touch bound regions, they just capture the in-scope
        // type/region parameters
        match obligation.self_ty().skip_binder().sty {
            ty::TyClosure(closure_def_id, closure_substs) => {
                debug!("assemble_unboxed_candidates: kind={:?} obligation={:?}",
                       kind, obligation);
                match self.infcx.closure_kind(closure_def_id, closure_substs) {
                    Some(closure_kind) => {
                        debug!("assemble_unboxed_candidates: closure_kind = {:?}", closure_kind);
                        if closure_kind.extends(kind) {
                            candidates.vec.push(ClosureCandidate);
                        }
                    }
                    None => {
                        debug!("assemble_unboxed_candidates: closure_kind not yet known");
                        candidates.vec.push(ClosureCandidate);
                    }
                };
                Ok(())
            }
            ty::TyInfer(ty::TyVar(_)) => {
                debug!("assemble_unboxed_closure_candidates: ambiguous self-type");
                candidates.ambiguous = true;
                return Ok(());
            }
            _ => { return Ok(()); }
        }
    }

    /// Implement one of the `Fn()` family for a fn pointer.
    fn assemble_fn_pointer_candidates(&mut self,
                                      obligation: &TraitObligation<'tcx>,
                                      candidates: &mut SelectionCandidateSet<'tcx>)
                                      -> Result<(),SelectionError<'tcx>>
    {
        // We provide impl of all fn traits for fn pointers.
        if self.tcx().lang_items().fn_trait_kind(obligation.predicate.def_id()).is_none() {
            return Ok(());
        }

        // ok to skip binder because what we are inspecting doesn't involve bound regions
        let self_ty = *obligation.self_ty().skip_binder();
        match self_ty.sty {
            ty::TyInfer(ty::TyVar(_)) => {
                debug!("assemble_fn_pointer_candidates: ambiguous self-type");
                candidates.ambiguous = true; // could wind up being a fn() type
            }

            // provide an impl, but only for suitable `fn` pointers
            ty::TyFnDef(..) | ty::TyFnPtr(_) => {
                if let ty::FnSig {
                    unsafety: hir::Unsafety::Normal,
                    abi: Abi::Rust,
                    variadic: false,
                    ..
                } = self_ty.fn_sig(self.tcx()).skip_binder() {
                    candidates.vec.push(FnPointerCandidate);
                }
            }

            _ => { }
        }

        Ok(())
    }

    /// Search for impls that might apply to `obligation`.
    fn assemble_candidates_from_impls(&mut self,
                                      obligation: &TraitObligation<'tcx>,
                                      candidates: &mut SelectionCandidateSet<'tcx>)
                                      -> Result<(), SelectionError<'tcx>>
    {
        debug!("assemble_candidates_from_impls(obligation={:?})", obligation);

        self.tcx().for_each_relevant_impl(
            obligation.predicate.def_id(),
            obligation.predicate.skip_binder().trait_ref.self_ty(),
            |impl_def_id| {
                self.probe(|this, snapshot| { /* [1] */
                    match this.match_impl(impl_def_id, obligation, snapshot) {
                        Ok(skol_map) => {
                            candidates.vec.push(ImplCandidate(impl_def_id));

                            // NB: we can safely drop the skol map
                            // since we are in a probe [1]
                            mem::drop(skol_map);
                        }
                        Err(_) => { }
                    }
                });
            }
        );

        Ok(())
    }

    fn assemble_candidates_from_auto_impls(&mut self,
                                              obligation: &TraitObligation<'tcx>,
                                              candidates: &mut SelectionCandidateSet<'tcx>)
                                              -> Result<(), SelectionError<'tcx>>
    {
        // OK to skip binder here because the tests we do below do not involve bound regions
        let self_ty = *obligation.self_ty().skip_binder();
        debug!("assemble_candidates_from_auto_impls(self_ty={:?})", self_ty);

        let def_id = obligation.predicate.def_id();

        if self.tcx().trait_is_auto(def_id) {
            match self_ty.sty {
                ty::TyDynamic(..) => {
                    // For object types, we don't know what the closed
                    // over types are. This means we conservatively
                    // say nothing; a candidate may be added by
                    // `assemble_candidates_from_object_ty`.
                }
                ty::TyForeign(..) => {
                    // Since the contents of foreign types is unknown,
                    // we don't add any `..` impl. Default traits could
                    // still be provided by a manual implementation for
                    // this trait and type.
                }
                ty::TyParam(..) |
                ty::TyProjection(..) => {
                    // In these cases, we don't know what the actual
                    // type is.  Therefore, we cannot break it down
                    // into its constituent types. So we don't
                    // consider the `..` impl but instead just add no
                    // candidates: this means that typeck will only
                    // succeed if there is another reason to believe
                    // that this obligation holds. That could be a
                    // where-clause or, in the case of an object type,
                    // it could be that the object type lists the
                    // trait (e.g. `Foo+Send : Send`). See
                    // `compile-fail/typeck-default-trait-impl-send-param.rs`
                    // for an example of a test case that exercises
                    // this path.
                }
                ty::TyInfer(ty::TyVar(_)) => {
                    // the auto impl might apply, we don't know
                    candidates.ambiguous = true;
                }
                _ => {
                    candidates.vec.push(AutoImplCandidate(def_id.clone()))
                }
            }
        }

        Ok(())
    }

    /// Search for impls that might apply to `obligation`.
    fn assemble_candidates_from_object_ty(&mut self,
                                          obligation: &TraitObligation<'tcx>,
                                          candidates: &mut SelectionCandidateSet<'tcx>)
    {
        debug!("assemble_candidates_from_object_ty(self_ty={:?})",
               obligation.self_ty().skip_binder());

        // Object-safety candidates are only applicable to object-safe
        // traits. Including this check is useful because it helps
        // inference in cases of traits like `BorrowFrom`, which are
        // not object-safe, and which rely on being able to infer the
        // self-type from one of the other inputs. Without this check,
        // these cases wind up being considered ambiguous due to a
        // (spurious) ambiguity introduced here.
        let predicate_trait_ref = obligation.predicate.to_poly_trait_ref();
        if !self.tcx().is_object_safe(predicate_trait_ref.def_id()) {
            return;
        }

        self.probe(|this, _snapshot| {
            // the code below doesn't care about regions, and the
            // self-ty here doesn't escape this probe, so just erase
            // any LBR.
            let self_ty = this.tcx().erase_late_bound_regions(&obligation.self_ty());
            let poly_trait_ref = match self_ty.sty {
                ty::TyDynamic(ref data, ..) => {
                    if data.auto_traits().any(|did| did == obligation.predicate.def_id()) {
                        debug!("assemble_candidates_from_object_ty: matched builtin bound, \
                                    pushing candidate");
                        candidates.vec.push(BuiltinObjectCandidate);
                        return;
                    }

                    match data.principal() {
                        Some(p) => p.with_self_ty(this.tcx(), self_ty),
                        None => return,
                    }
                }
                ty::TyInfer(ty::TyVar(_)) => {
                    debug!("assemble_candidates_from_object_ty: ambiguous");
                    candidates.ambiguous = true; // could wind up being an object type
                    return;
                }
                _ => {
                    return;
                }
            };

            debug!("assemble_candidates_from_object_ty: poly_trait_ref={:?}",
                   poly_trait_ref);

            // Count only those upcast versions that match the trait-ref
            // we are looking for. Specifically, do not only check for the
            // correct trait, but also the correct type parameters.
            // For example, we may be trying to upcast `Foo` to `Bar<i32>`,
            // but `Foo` is declared as `trait Foo : Bar<u32>`.
            let upcast_trait_refs =
                util::supertraits(this.tcx(), poly_trait_ref)
                .filter(|upcast_trait_ref| {
                    this.probe(|this, _| {
                        let upcast_trait_ref = upcast_trait_ref.clone();
                        this.match_poly_trait_ref(obligation, upcast_trait_ref).is_ok()
                    })
                })
                .count();

            if upcast_trait_refs > 1 {
                // can be upcast in many ways; need more type information
                candidates.ambiguous = true;
            } else if upcast_trait_refs == 1 {
                candidates.vec.push(ObjectCandidate);
            }
        })
    }

    /// Search for unsizing that might apply to `obligation`.
    fn assemble_candidates_for_unsizing(&mut self,
                                        obligation: &TraitObligation<'tcx>,
                                        candidates: &mut SelectionCandidateSet<'tcx>) {
        // We currently never consider higher-ranked obligations e.g.
        // `for<'a> &'a T: Unsize<Trait+'a>` to be implemented. This is not
        // because they are a priori invalid, and we could potentially add support
        // for them later, it's just that there isn't really a strong need for it.
        // A `T: Unsize<U>` obligation is always used as part of a `T: CoerceUnsize<U>`
        // impl, and those are generally applied to concrete types.
        //
        // That said, one might try to write a fn with a where clause like
        //     for<'a> Foo<'a, T>: Unsize<Foo<'a, Trait>>
        // where the `'a` is kind of orthogonal to the relevant part of the `Unsize`.
        // Still, you'd be more likely to write that where clause as
        //     T: Trait
        // so it seems ok if we (conservatively) fail to accept that `Unsize`
        // obligation above. Should be possible to extend this in the future.
        let source = match obligation.self_ty().no_late_bound_regions() {
            Some(t) => t,
            None => {
                // Don't add any candidates if there are bound regions.
                return;
            }
        };
        let target = obligation.predicate.skip_binder().trait_ref.substs.type_at(1);

        debug!("assemble_candidates_for_unsizing(source={:?}, target={:?})",
               source, target);

        let may_apply = match (&source.sty, &target.sty) {
            // Trait+Kx+'a -> Trait+Ky+'b (upcasts).
            (&ty::TyDynamic(ref data_a, ..), &ty::TyDynamic(ref data_b, ..)) => {
                // Upcasts permit two things:
                //
                // 1. Dropping builtin bounds, e.g. `Foo+Send` to `Foo`
                // 2. Tightening the region bound, e.g. `Foo+'a` to `Foo+'b` if `'a : 'b`
                //
                // Note that neither of these changes requires any
                // change at runtime.  Eventually this will be
                // generalized.
                //
                // We always upcast when we can because of reason
                // #2 (region bounds).
                match (data_a.principal(), data_b.principal()) {
                    (Some(a), Some(b)) => a.def_id() == b.def_id() &&
                        data_b.auto_traits()
                            // All of a's auto traits need to be in b's auto traits.
                            .all(|b| data_a.auto_traits().any(|a| a == b)),
                    _ => false
                }
            }

            // T -> Trait.
            (_, &ty::TyDynamic(..)) => true,

            // Ambiguous handling is below T -> Trait, because inference
            // variables can still implement Unsize<Trait> and nested
            // obligations will have the final say (likely deferred).
            (&ty::TyInfer(ty::TyVar(_)), _) |
            (_, &ty::TyInfer(ty::TyVar(_))) => {
                debug!("assemble_candidates_for_unsizing: ambiguous");
                candidates.ambiguous = true;
                false
            }

            // [T; n] -> [T].
            (&ty::TyArray(..), &ty::TySlice(_)) => true,

            // Struct<T> -> Struct<U>.
            (&ty::TyAdt(def_id_a, _), &ty::TyAdt(def_id_b, _)) if def_id_a.is_struct() => {
                def_id_a == def_id_b
            }

            // (.., T) -> (.., U).
            (&ty::TyTuple(tys_a), &ty::TyTuple(tys_b)) => {
                tys_a.len() == tys_b.len()
            }

            _ => false
        };

        if may_apply {
            candidates.vec.push(BuiltinUnsizeCandidate);
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // WINNOW
    //
    // Winnowing is the process of attempting to resolve ambiguity by
    // probing further. During the winnowing process, we unify all
    // type variables (ignoring skolemization) and then we also
    // attempt to evaluate recursive bounds to see if they are
    // satisfied.

    /// Returns true if `victim` should be dropped in favor of
    /// `other`.  Generally speaking we will drop duplicate
    /// candidates and prefer where-clause candidates.
    ///
    /// See the comment for "SelectionCandidate" for more details.
    fn candidate_should_be_dropped_in_favor_of<'o>(
        &mut self,
        victim: &EvaluatedCandidate<'tcx>,
        other: &EvaluatedCandidate<'tcx>)
        -> bool
    {
        // Check if a bound would previously have been removed when normalizing
        // the param_env so that it can be given the lowest priority. See
        // #50825 for the motivation for this.
        let is_global = |cand: &ty::PolyTraitRef<'_>| {
            cand.is_global() && !cand.has_late_bound_regions()
        };

        if victim.candidate == other.candidate {
            return true;
        }

        match other.candidate {
            ParamCandidate(ref cand) => match victim.candidate {
                AutoImplCandidate(..) => {
                    bug!(
                        "default implementations shouldn't be recorded \
                         when there are other valid candidates");
                }
                ImplCandidate(..) |
                ClosureCandidate |
                GeneratorCandidate |
                FnPointerCandidate |
                BuiltinObjectCandidate |
                BuiltinUnsizeCandidate |
                BuiltinCandidate { .. } => {
                    // Global bounds from the where clause should be ignored
                    // here (see issue #50825). Otherwise, we have a where
                    // clause so don't go around looking for impls.
                    !is_global(cand)
                }
                ObjectCandidate |
                ProjectionCandidate => {
                    // Arbitrarily give param candidates priority
                    // over projection and object candidates.
                    !is_global(cand)
                },
                ParamCandidate(..) => false,
            },
            ObjectCandidate |
            ProjectionCandidate => match victim.candidate {
                AutoImplCandidate(..) => {
                    bug!(
                        "default implementations shouldn't be recorded \
                         when there are other valid candidates");
                }
                ImplCandidate(..) |
                ClosureCandidate |
                GeneratorCandidate |
                FnPointerCandidate |
                BuiltinObjectCandidate |
                BuiltinUnsizeCandidate |
                BuiltinCandidate { .. } => {
                    true
                }
                ObjectCandidate |
                ProjectionCandidate => {
                    // Arbitrarily give param candidates priority
                    // over projection and object candidates.
                    true
                },
                ParamCandidate(ref cand) => is_global(cand),
            },
            ImplCandidate(other_def) => {
                // See if we can toss out `victim` based on specialization.
                // This requires us to know *for sure* that the `other` impl applies
                // i.e. EvaluatedToOk:
                if other.evaluation == EvaluatedToOk {
                    match victim.candidate {
                        ImplCandidate(victim_def) => {
                            let tcx = self.tcx().global_tcx();
                            return tcx.specializes((other_def, victim_def)) ||
                                tcx.impls_are_allowed_to_overlap(other_def, victim_def);
                        }
                        ParamCandidate(ref cand) => {
                            // Prefer the impl to a global where clause candidate.
                            return is_global(cand);
                        }
                        _ => ()
                    }
                }

                false
            },
            ClosureCandidate |
            GeneratorCandidate |
            FnPointerCandidate |
            BuiltinObjectCandidate |
            BuiltinUnsizeCandidate |
            BuiltinCandidate { .. } => {
                match victim.candidate {
                    ParamCandidate(ref cand) => {
                        // Prefer these to a global where-clause bound
                        // (see issue #50825)
                        is_global(cand) && other.evaluation == EvaluatedToOk
                    }
                    _ => false,
                }
            }
            _ => false
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // BUILTIN BOUNDS
    //
    // These cover the traits that are built-in to the language
    // itself: `Copy`, `Clone` and `Sized`.

    fn assemble_builtin_bound_candidates<'o>(&mut self,
                                             conditions: BuiltinImplConditions<'tcx>,
                                             candidates: &mut SelectionCandidateSet<'tcx>)
                                             -> Result<(),SelectionError<'tcx>>
    {
        match conditions {
            BuiltinImplConditions::Where(nested) => {
                debug!("builtin_bound: nested={:?}", nested);
                candidates.vec.push(BuiltinCandidate {
                    has_nested: nested.skip_binder().len() > 0
                });
                Ok(())
            }
            BuiltinImplConditions::None => { Ok(()) }
            BuiltinImplConditions::Ambiguous => {
                debug!("assemble_builtin_bound_candidates: ambiguous builtin");
                Ok(candidates.ambiguous = true)
            }
        }
    }

    fn sized_conditions(&mut self, obligation: &TraitObligation<'tcx>)
                     -> BuiltinImplConditions<'tcx>
    {
        use self::BuiltinImplConditions::{Ambiguous, None, Where};

        // NOTE: binder moved to (*)
        let self_ty = self.infcx.shallow_resolve(
            obligation.predicate.skip_binder().self_ty());

        match self_ty.sty {
            ty::TyInfer(ty::IntVar(_)) | ty::TyInfer(ty::FloatVar(_)) |
            ty::TyUint(_) | ty::TyInt(_) | ty::TyBool | ty::TyFloat(_) |
            ty::TyFnDef(..) | ty::TyFnPtr(_) | ty::TyRawPtr(..) |
            ty::TyChar | ty::TyRef(..) | ty::TyGenerator(..) |
            ty::TyGeneratorWitness(..) | ty::TyArray(..) | ty::TyClosure(..) |
            ty::TyNever | ty::TyError => {
                // safe for everything
                Where(ty::Binder::dummy(Vec::new()))
            }

            ty::TyStr | ty::TySlice(_) | ty::TyDynamic(..) | ty::TyForeign(..) => None,

            ty::TyTuple(tys) => {
                Where(ty::Binder::bind(tys.last().into_iter().cloned().collect()))
            }

            ty::TyAdt(def, substs) => {
                let sized_crit = def.sized_constraint(self.tcx());
                // (*) binder moved here
                Where(ty::Binder::bind(
                    sized_crit.iter().map(|ty| ty.subst(self.tcx(), substs)).collect()
                ))
            }

            ty::TyProjection(_) | ty::TyParam(_) | ty::TyAnon(..) => None,
            ty::TyInfer(ty::TyVar(_)) => Ambiguous,

            ty::TyInfer(ty::CanonicalTy(_)) |
            ty::TyInfer(ty::FreshTy(_)) |
            ty::TyInfer(ty::FreshIntTy(_)) |
            ty::TyInfer(ty::FreshFloatTy(_)) => {
                bug!("asked to assemble builtin bounds of unexpected type: {:?}",
                     self_ty);
            }
        }
    }

    fn copy_clone_conditions(&mut self, obligation: &TraitObligation<'tcx>)
                     -> BuiltinImplConditions<'tcx>
    {
        // NOTE: binder moved to (*)
        let self_ty = self.infcx.shallow_resolve(
            obligation.predicate.skip_binder().self_ty());

        use self::BuiltinImplConditions::{Ambiguous, None, Where};

        match self_ty.sty {
            ty::TyInfer(ty::IntVar(_)) | ty::TyInfer(ty::FloatVar(_)) |
            ty::TyFnDef(..) | ty::TyFnPtr(_) | ty::TyError => {
                Where(ty::Binder::dummy(Vec::new()))
            }

            ty::TyUint(_) | ty::TyInt(_) | ty::TyBool | ty::TyFloat(_) |
            ty::TyChar | ty::TyRawPtr(..) | ty::TyNever |
            ty::TyRef(_, _, hir::MutImmutable) => {
                // Implementations provided in libcore
                None
            }

            ty::TyDynamic(..) | ty::TyStr | ty::TySlice(..) |
            ty::TyGenerator(..) | ty::TyGeneratorWitness(..) | ty::TyForeign(..) |
            ty::TyRef(_, _, hir::MutMutable) => {
                None
            }

            ty::TyArray(element_ty, _) => {
                // (*) binder moved here
                Where(ty::Binder::bind(vec![element_ty]))
            }

            ty::TyTuple(tys) => {
                // (*) binder moved here
                Where(ty::Binder::bind(tys.to_vec()))
            }

            ty::TyClosure(def_id, substs) => {
                let trait_id = obligation.predicate.def_id();
                let is_copy_trait = Some(trait_id) == self.tcx().lang_items().copy_trait();
                let is_clone_trait = Some(trait_id) == self.tcx().lang_items().clone_trait();
                if is_copy_trait || is_clone_trait {
                    Where(ty::Binder::bind(substs.upvar_tys(def_id, self.tcx()).collect()))
                } else {
                    None
                }
            }

            ty::TyAdt(..) | ty::TyProjection(..) | ty::TyParam(..) | ty::TyAnon(..) => {
                // Fallback to whatever user-defined impls exist in this case.
                None
            }

            ty::TyInfer(ty::TyVar(_)) => {
                // Unbound type variable. Might or might not have
                // applicable impls and so forth, depending on what
                // those type variables wind up being bound to.
                Ambiguous
            }

            ty::TyInfer(ty::CanonicalTy(_)) |
            ty::TyInfer(ty::FreshTy(_)) |
            ty::TyInfer(ty::FreshIntTy(_)) |
            ty::TyInfer(ty::FreshFloatTy(_)) => {
                bug!("asked to assemble builtin bounds of unexpected type: {:?}",
                     self_ty);
            }
        }
    }

    /// For default impls, we need to break apart a type into its
    /// "constituent types" -- meaning, the types that it contains.
    ///
    /// Here are some (simple) examples:
    ///
    /// ```
    /// (i32, u32) -> [i32, u32]
    /// Foo where struct Foo { x: i32, y: u32 } -> [i32, u32]
    /// Bar<i32> where struct Bar<T> { x: T, y: u32 } -> [i32, u32]
    /// Zed<i32> where enum Zed { A(T), B(u32) } -> [i32, u32]
    /// ```
    fn constituent_types_for_ty(&self, t: Ty<'tcx>) -> Vec<Ty<'tcx>> {
        match t.sty {
            ty::TyUint(_) |
            ty::TyInt(_) |
            ty::TyBool |
            ty::TyFloat(_) |
            ty::TyFnDef(..) |
            ty::TyFnPtr(_) |
            ty::TyStr |
            ty::TyError |
            ty::TyInfer(ty::IntVar(_)) |
            ty::TyInfer(ty::FloatVar(_)) |
            ty::TyNever |
            ty::TyChar => {
                Vec::new()
            }

            ty::TyDynamic(..) |
            ty::TyParam(..) |
            ty::TyForeign(..) |
            ty::TyProjection(..) |
            ty::TyInfer(ty::CanonicalTy(_)) |
            ty::TyInfer(ty::TyVar(_)) |
            ty::TyInfer(ty::FreshTy(_)) |
            ty::TyInfer(ty::FreshIntTy(_)) |
            ty::TyInfer(ty::FreshFloatTy(_)) => {
                bug!("asked to assemble constituent types of unexpected type: {:?}",
                     t);
            }

            ty::TyRawPtr(ty::TypeAndMut { ty: element_ty, ..}) |
            ty::TyRef(_, element_ty, _) => {
                vec![element_ty]
            },

            ty::TyArray(element_ty, _) | ty::TySlice(element_ty) => {
                vec![element_ty]
            }

            ty::TyTuple(ref tys) => {
                // (T1, ..., Tn) -- meets any bound that all of T1...Tn meet
                tys.to_vec()
            }

            ty::TyClosure(def_id, ref substs) => {
                substs.upvar_tys(def_id, self.tcx()).collect()
            }

            ty::TyGenerator(def_id, ref substs, _) => {
                let witness = substs.witness(def_id, self.tcx());
                substs.upvar_tys(def_id, self.tcx()).chain(iter::once(witness)).collect()
            }

            ty::TyGeneratorWitness(types) => {
                // This is sound because no regions in the witness can refer to
                // the binder outside the witness. So we'll effectivly reuse
                // the implicit binder around the witness.
                types.skip_binder().to_vec()
            }

            // for `PhantomData<T>`, we pass `T`
            ty::TyAdt(def, substs) if def.is_phantom_data() => {
                substs.types().collect()
            }

            ty::TyAdt(def, substs) => {
                def.all_fields()
                    .map(|f| f.ty(self.tcx(), substs))
                    .collect()
            }

            ty::TyAnon(def_id, substs) => {
                // We can resolve the `impl Trait` to its concrete type,
                // which enforces a DAG between the functions requiring
                // the auto trait bounds in question.
                vec![self.tcx().type_of(def_id).subst(self.tcx(), substs)]
            }
        }
    }

    fn collect_predicates_for_types(&mut self,
                                    param_env: ty::ParamEnv<'tcx>,
                                    cause: ObligationCause<'tcx>,
                                    recursion_depth: usize,
                                    trait_def_id: DefId,
                                    types: ty::Binder<Vec<Ty<'tcx>>>)
                                    -> Vec<PredicateObligation<'tcx>>
    {
        // Because the types were potentially derived from
        // higher-ranked obligations they may reference late-bound
        // regions. For example, `for<'a> Foo<&'a int> : Copy` would
        // yield a type like `for<'a> &'a int`. In general, we
        // maintain the invariant that we never manipulate bound
        // regions, so we have to process these bound regions somehow.
        //
        // The strategy is to:
        //
        // 1. Instantiate those regions to skolemized regions (e.g.,
        //    `for<'a> &'a int` becomes `&0 int`.
        // 2. Produce something like `&'0 int : Copy`
        // 3. Re-bind the regions back to `for<'a> &'a int : Copy`

        types.skip_binder().into_iter().flat_map(|ty| { // binder moved -\
            let ty: ty::Binder<Ty<'tcx>> = ty::Binder::bind(ty); // <----/

            self.in_snapshot(|this, snapshot| {
                let (skol_ty, skol_map) =
                    this.infcx().skolemize_late_bound_regions(&ty);
                let Normalized { value: normalized_ty, mut obligations } =
                    project::normalize_with_depth(this,
                                                  param_env,
                                                  cause.clone(),
                                                  recursion_depth,
                                                  &skol_ty);
                let skol_obligation =
                    this.tcx().predicate_for_trait_def(param_env,
                                                       cause.clone(),
                                                       trait_def_id,
                                                       recursion_depth,
                                                       normalized_ty,
                                                       &[]);
                obligations.push(skol_obligation);
                this.infcx().plug_leaks(skol_map, snapshot, obligations)
            })
        }).collect()
    }

    ///////////////////////////////////////////////////////////////////////////
    // CONFIRMATION
    //
    // Confirmation unifies the output type parameters of the trait
    // with the values found in the obligation, possibly yielding a
    // type error.  See [rustc guide] for more details.
    //
    // [rustc guide]:
    // https://rust-lang-nursery.github.io/rustc-guide/trait-resolution.html#confirmation

    fn confirm_candidate(&mut self,
                         obligation: &TraitObligation<'tcx>,
                         candidate: SelectionCandidate<'tcx>)
                         -> Result<Selection<'tcx>,SelectionError<'tcx>>
    {
        debug!("confirm_candidate({:?}, {:?})",
               obligation,
               candidate);

        match candidate {
            BuiltinCandidate { has_nested } => {
                let data = self.confirm_builtin_candidate(obligation, has_nested);
                Ok(VtableBuiltin(data))
            }

            ParamCandidate(param) => {
                let obligations = self.confirm_param_candidate(obligation, param);
                Ok(VtableParam(obligations))
            }

            AutoImplCandidate(trait_def_id) => {
                let data = self.confirm_auto_impl_candidate(obligation, trait_def_id);
                Ok(VtableAutoImpl(data))
            }

            ImplCandidate(impl_def_id) => {
                Ok(VtableImpl(self.confirm_impl_candidate(obligation, impl_def_id)))
            }

            ClosureCandidate => {
                let vtable_closure = self.confirm_closure_candidate(obligation)?;
                Ok(VtableClosure(vtable_closure))
            }

            GeneratorCandidate => {
                let vtable_generator = self.confirm_generator_candidate(obligation)?;
                Ok(VtableGenerator(vtable_generator))
            }

            BuiltinObjectCandidate => {
                // This indicates something like `(Trait+Send) :
                // Send`. In this case, we know that this holds
                // because that's what the object type is telling us,
                // and there's really no additional obligations to
                // prove and no types in particular to unify etc.
                Ok(VtableParam(Vec::new()))
            }

            ObjectCandidate => {
                let data = self.confirm_object_candidate(obligation);
                Ok(VtableObject(data))
            }

            FnPointerCandidate => {
                let data =
                    self.confirm_fn_pointer_candidate(obligation)?;
                Ok(VtableFnPointer(data))
            }

            ProjectionCandidate => {
                self.confirm_projection_candidate(obligation);
                Ok(VtableParam(Vec::new()))
            }

            BuiltinUnsizeCandidate => {
                let data = self.confirm_builtin_unsize_candidate(obligation)?;
                Ok(VtableBuiltin(data))
            }
        }
    }

    fn confirm_projection_candidate(&mut self,
                                    obligation: &TraitObligation<'tcx>)
    {
        self.in_snapshot(|this, snapshot| {
            let result =
                this.match_projection_obligation_against_definition_bounds(obligation,
                                                                           snapshot);
            assert!(result);
        })
    }

    fn confirm_param_candidate(&mut self,
                               obligation: &TraitObligation<'tcx>,
                               param: ty::PolyTraitRef<'tcx>)
                               -> Vec<PredicateObligation<'tcx>>
    {
        debug!("confirm_param_candidate({:?},{:?})",
               obligation,
               param);

        // During evaluation, we already checked that this
        // where-clause trait-ref could be unified with the obligation
        // trait-ref. Repeat that unification now without any
        // transactional boundary; it should not fail.
        match self.match_where_clause_trait_ref(obligation, param.clone()) {
            Ok(obligations) => obligations,
            Err(()) => {
                bug!("Where clause `{:?}` was applicable to `{:?}` but now is not",
                     param,
                     obligation);
            }
        }
    }

    fn confirm_builtin_candidate(&mut self,
                                 obligation: &TraitObligation<'tcx>,
                                 has_nested: bool)
                                 -> VtableBuiltinData<PredicateObligation<'tcx>>
    {
        debug!("confirm_builtin_candidate({:?}, {:?})",
               obligation, has_nested);

        let lang_items = self.tcx().lang_items();
        let obligations = if has_nested {
            let trait_def = obligation.predicate.def_id();
            let conditions = match trait_def {
                _ if Some(trait_def) == lang_items.sized_trait() => {
                    self.sized_conditions(obligation)
                }
                _ if Some(trait_def) == lang_items.copy_trait() => {
                    self.copy_clone_conditions(obligation)
                }
                _ if Some(trait_def) == lang_items.clone_trait() => {
                    self.copy_clone_conditions(obligation)
                }
                _ => bug!("unexpected builtin trait {:?}", trait_def)
            };
            let nested = match conditions {
                BuiltinImplConditions::Where(nested) => nested,
                _ => bug!("obligation {:?} had matched a builtin impl but now doesn't",
                          obligation)
            };

            let cause = obligation.derived_cause(BuiltinDerivedObligation);
            self.collect_predicates_for_types(obligation.param_env,
                                              cause,
                                              obligation.recursion_depth+1,
                                              trait_def,
                                              nested)
        } else {
            vec![]
        };

        debug!("confirm_builtin_candidate: obligations={:?}",
               obligations);

        VtableBuiltinData { nested: obligations }
    }

    /// This handles the case where a `auto trait Foo` impl is being used.
    /// The idea is that the impl applies to `X : Foo` if the following conditions are met:
    ///
    /// 1. For each constituent type `Y` in `X`, `Y : Foo` holds
    /// 2. For each where-clause `C` declared on `Foo`, `[Self => X] C` holds.
    fn confirm_auto_impl_candidate(&mut self,
                                   obligation: &TraitObligation<'tcx>,
                                   trait_def_id: DefId)
                                   -> VtableAutoImplData<PredicateObligation<'tcx>>
    {
        debug!("confirm_auto_impl_candidate({:?}, {:?})",
               obligation,
               trait_def_id);

        let types = obligation.predicate.map_bound(|inner| {
            let self_ty = self.infcx.shallow_resolve(inner.self_ty());
            self.constituent_types_for_ty(self_ty)
        });
        self.vtable_auto_impl(obligation, trait_def_id, types)
    }

    /// See `confirm_auto_impl_candidate`
    fn vtable_auto_impl(&mut self,
                           obligation: &TraitObligation<'tcx>,
                           trait_def_id: DefId,
                           nested: ty::Binder<Vec<Ty<'tcx>>>)
                           -> VtableAutoImplData<PredicateObligation<'tcx>>
    {
        debug!("vtable_auto_impl: nested={:?}", nested);

        let cause = obligation.derived_cause(BuiltinDerivedObligation);
        let mut obligations = self.collect_predicates_for_types(
            obligation.param_env,
            cause,
            obligation.recursion_depth+1,
            trait_def_id,
            nested);

        let trait_obligations = self.in_snapshot(|this, snapshot| {
            let poly_trait_ref = obligation.predicate.to_poly_trait_ref();
            let (trait_ref, skol_map) =
                this.infcx().skolemize_late_bound_regions(&poly_trait_ref);
            let cause = obligation.derived_cause(ImplDerivedObligation);
            this.impl_or_trait_obligations(cause,
                                           obligation.recursion_depth + 1,
                                           obligation.param_env,
                                           trait_def_id,
                                           &trait_ref.substs,
                                           skol_map,
                                           snapshot)
        });

        obligations.extend(trait_obligations);

        debug!("vtable_auto_impl: obligations={:?}", obligations);

        VtableAutoImplData {
            trait_def_id,
            nested: obligations
        }
    }

    fn confirm_impl_candidate(&mut self,
                              obligation: &TraitObligation<'tcx>,
                              impl_def_id: DefId)
                              -> VtableImplData<'tcx, PredicateObligation<'tcx>>
    {
        debug!("confirm_impl_candidate({:?},{:?})",
               obligation,
               impl_def_id);

        // First, create the substitutions by matching the impl again,
        // this time not in a probe.
        self.in_snapshot(|this, snapshot| {
            let (substs, skol_map) =
                this.rematch_impl(impl_def_id, obligation,
                                  snapshot);
            debug!("confirm_impl_candidate substs={:?}", substs);
            let cause = obligation.derived_cause(ImplDerivedObligation);
            this.vtable_impl(impl_def_id,
                             substs,
                             cause,
                             obligation.recursion_depth + 1,
                             obligation.param_env,
                             skol_map,
                             snapshot)
        })
    }

    fn vtable_impl(&mut self,
                   impl_def_id: DefId,
                   mut substs: Normalized<'tcx, &'tcx Substs<'tcx>>,
                   cause: ObligationCause<'tcx>,
                   recursion_depth: usize,
                   param_env: ty::ParamEnv<'tcx>,
                   skol_map: infer::SkolemizationMap<'tcx>,
                   snapshot: &infer::CombinedSnapshot<'cx, 'tcx>)
                   -> VtableImplData<'tcx, PredicateObligation<'tcx>>
    {
        debug!("vtable_impl(impl_def_id={:?}, substs={:?}, recursion_depth={}, skol_map={:?})",
               impl_def_id,
               substs,
               recursion_depth,
               skol_map);

        let mut impl_obligations =
            self.impl_or_trait_obligations(cause,
                                           recursion_depth,
                                           param_env,
                                           impl_def_id,
                                           &substs.value,
                                           skol_map,
                                           snapshot);

        debug!("vtable_impl: impl_def_id={:?} impl_obligations={:?}",
               impl_def_id,
               impl_obligations);

        // Because of RFC447, the impl-trait-ref and obligations
        // are sufficient to determine the impl substs, without
        // relying on projections in the impl-trait-ref.
        //
        // e.g. `impl<U: Tr, V: Iterator<Item=U>> Foo<<U as Tr>::T> for V`
        impl_obligations.append(&mut substs.obligations);

        VtableImplData { impl_def_id,
                         substs: substs.value,
                         nested: impl_obligations }
    }

    fn confirm_object_candidate(&mut self,
                                obligation: &TraitObligation<'tcx>)
                                -> VtableObjectData<'tcx, PredicateObligation<'tcx>>
    {
        debug!("confirm_object_candidate({:?})",
               obligation);

        // FIXME skipping binder here seems wrong -- we should
        // probably flatten the binder from the obligation and the
        // binder from the object. Have to try to make a broken test
        // case that results. -nmatsakis
        let self_ty = self.infcx.shallow_resolve(*obligation.self_ty().skip_binder());
        let poly_trait_ref = match self_ty.sty {
            ty::TyDynamic(ref data, ..) => {
                data.principal().unwrap().with_self_ty(self.tcx(), self_ty)
            }
            _ => {
                span_bug!(obligation.cause.span,
                          "object candidate with non-object");
            }
        };

        let mut upcast_trait_ref = None;
        let mut nested = vec![];
        let vtable_base;

        {
            let tcx = self.tcx();

            // We want to find the first supertrait in the list of
            // supertraits that we can unify with, and do that
            // unification. We know that there is exactly one in the list
            // where we can unify because otherwise select would have
            // reported an ambiguity. (When we do find a match, also
            // record it for later.)
            let nonmatching =
                util::supertraits(tcx, poly_trait_ref)
                .take_while(|&t| {
                    match
                        self.commit_if_ok(
                            |this, _| this.match_poly_trait_ref(obligation, t))
                    {
                        Ok(obligations) => {
                            upcast_trait_ref = Some(t);
                            nested.extend(obligations);
                            false
                        }
                        Err(_) => { true }
                    }
                });

            // Additionally, for each of the nonmatching predicates that
            // we pass over, we sum up the set of number of vtable
            // entries, so that we can compute the offset for the selected
            // trait.
            vtable_base =
                nonmatching.map(|t| tcx.count_own_vtable_entries(t))
                           .sum();

        }

        VtableObjectData {
            upcast_trait_ref: upcast_trait_ref.unwrap(),
            vtable_base,
            nested,
        }
    }

    fn confirm_fn_pointer_candidate(&mut self, obligation: &TraitObligation<'tcx>)
        -> Result<VtableFnPointerData<'tcx, PredicateObligation<'tcx>>, SelectionError<'tcx>>
    {
        debug!("confirm_fn_pointer_candidate({:?})",
               obligation);

        // ok to skip binder; it is reintroduced below
        let self_ty = self.infcx.shallow_resolve(*obligation.self_ty().skip_binder());
        let sig = self_ty.fn_sig(self.tcx());
        let trait_ref =
            self.tcx().closure_trait_ref_and_return_type(obligation.predicate.def_id(),
                                                         self_ty,
                                                         sig,
                                                         util::TupleArgumentsFlag::Yes)
            .map_bound(|(trait_ref, _)| trait_ref);

        let Normalized { value: trait_ref, obligations } =
            project::normalize_with_depth(self,
                                          obligation.param_env,
                                          obligation.cause.clone(),
                                          obligation.recursion_depth + 1,
                                          &trait_ref);

        self.confirm_poly_trait_refs(obligation.cause.clone(),
                                     obligation.param_env,
                                     obligation.predicate.to_poly_trait_ref(),
                                     trait_ref)?;
        Ok(VtableFnPointerData { fn_ty: self_ty, nested: obligations })
    }

    fn confirm_generator_candidate(&mut self,
                                   obligation: &TraitObligation<'tcx>)
                                   -> Result<VtableGeneratorData<'tcx, PredicateObligation<'tcx>>,
                                           SelectionError<'tcx>>
    {
        // ok to skip binder because the substs on generator types never
        // touch bound regions, they just capture the in-scope
        // type/region parameters
        let self_ty = self.infcx.shallow_resolve(obligation.self_ty().skip_binder());
        let (generator_def_id, substs) = match self_ty.sty {
            ty::TyGenerator(id, substs, _) => (id, substs),
            _ => bug!("closure candidate for non-closure {:?}", obligation)
        };

        debug!("confirm_generator_candidate({:?},{:?},{:?})",
               obligation,
               generator_def_id,
               substs);

        let trait_ref =
            self.generator_trait_ref_unnormalized(obligation, generator_def_id, substs);
        let Normalized {
            value: trait_ref,
            mut obligations
        } = normalize_with_depth(self,
                                 obligation.param_env,
                                 obligation.cause.clone(),
                                 obligation.recursion_depth+1,
                                 &trait_ref);

        debug!("confirm_generator_candidate(generator_def_id={:?}, \
                trait_ref={:?}, obligations={:?})",
               generator_def_id,
               trait_ref,
               obligations);

        obligations.extend(
            self.confirm_poly_trait_refs(obligation.cause.clone(),
                                        obligation.param_env,
                                        obligation.predicate.to_poly_trait_ref(),
                                        trait_ref)?);

        Ok(VtableGeneratorData {
            generator_def_id: generator_def_id,
            substs: substs.clone(),
            nested: obligations
        })
    }

    fn confirm_closure_candidate(&mut self,
                                 obligation: &TraitObligation<'tcx>)
                                 -> Result<VtableClosureData<'tcx, PredicateObligation<'tcx>>,
                                           SelectionError<'tcx>>
    {
        debug!("confirm_closure_candidate({:?})", obligation);

        let kind = match self.tcx().lang_items().fn_trait_kind(obligation.predicate.def_id()) {
            Some(k) => k,
            None => bug!("closure candidate for non-fn trait {:?}", obligation)
        };

        // ok to skip binder because the substs on closure types never
        // touch bound regions, they just capture the in-scope
        // type/region parameters
        let self_ty = self.infcx.shallow_resolve(obligation.self_ty().skip_binder());
        let (closure_def_id, substs) = match self_ty.sty {
            ty::TyClosure(id, substs) => (id, substs),
            _ => bug!("closure candidate for non-closure {:?}", obligation)
        };

        let trait_ref =
            self.closure_trait_ref_unnormalized(obligation, closure_def_id, substs);
        let Normalized {
            value: trait_ref,
            mut obligations
        } = normalize_with_depth(self,
                                 obligation.param_env,
                                 obligation.cause.clone(),
                                 obligation.recursion_depth+1,
                                 &trait_ref);

        debug!("confirm_closure_candidate(closure_def_id={:?}, trait_ref={:?}, obligations={:?})",
               closure_def_id,
               trait_ref,
               obligations);

        obligations.extend(
            self.confirm_poly_trait_refs(obligation.cause.clone(),
                                        obligation.param_env,
                                        obligation.predicate.to_poly_trait_ref(),
                                        trait_ref)?);

        obligations.push(Obligation::new(
            obligation.cause.clone(),
            obligation.param_env,
            ty::Predicate::ClosureKind(closure_def_id, substs, kind)));

        Ok(VtableClosureData {
            closure_def_id,
            substs: substs.clone(),
            nested: obligations
        })
    }

    /// In the case of closure types and fn pointers,
    /// we currently treat the input type parameters on the trait as
    /// outputs. This means that when we have a match we have only
    /// considered the self type, so we have to go back and make sure
    /// to relate the argument types too.  This is kind of wrong, but
    /// since we control the full set of impls, also not that wrong,
    /// and it DOES yield better error messages (since we don't report
    /// errors as if there is no applicable impl, but rather report
    /// errors are about mismatched argument types.
    ///
    /// Here is an example. Imagine we have a closure expression
    /// and we desugared it so that the type of the expression is
    /// `Closure`, and `Closure` expects an int as argument. Then it
    /// is "as if" the compiler generated this impl:
    ///
    ///     impl Fn(int) for Closure { ... }
    ///
    /// Now imagine our obligation is `Fn(usize) for Closure`. So far
    /// we have matched the self-type `Closure`. At this point we'll
    /// compare the `int` to `usize` and generate an error.
    ///
    /// Note that this checking occurs *after* the impl has selected,
    /// because these output type parameters should not affect the
    /// selection of the impl. Therefore, if there is a mismatch, we
    /// report an error to the user.
    fn confirm_poly_trait_refs(&mut self,
                               obligation_cause: ObligationCause<'tcx>,
                               obligation_param_env: ty::ParamEnv<'tcx>,
                               obligation_trait_ref: ty::PolyTraitRef<'tcx>,
                               expected_trait_ref: ty::PolyTraitRef<'tcx>)
                               -> Result<Vec<PredicateObligation<'tcx>>, SelectionError<'tcx>>
    {
        let obligation_trait_ref = obligation_trait_ref.clone();
        self.infcx
            .at(&obligation_cause, obligation_param_env)
            .sup(obligation_trait_ref, expected_trait_ref)
            .map(|InferOk { obligations, .. }| obligations)
            .map_err(|e| OutputTypeParameterMismatch(expected_trait_ref, obligation_trait_ref, e))
    }

    fn confirm_builtin_unsize_candidate(&mut self,
                                        obligation: &TraitObligation<'tcx>,)
        -> Result<VtableBuiltinData<PredicateObligation<'tcx>>, SelectionError<'tcx>>
    {
        let tcx = self.tcx();

        // assemble_candidates_for_unsizing should ensure there are no late bound
        // regions here. See the comment there for more details.
        let source = self.infcx.shallow_resolve(
            obligation.self_ty().no_late_bound_regions().unwrap());
        let target = obligation.predicate.skip_binder().trait_ref.substs.type_at(1);
        let target = self.infcx.shallow_resolve(target);

        debug!("confirm_builtin_unsize_candidate(source={:?}, target={:?})",
               source, target);

        let mut nested = vec![];
        match (&source.sty, &target.sty) {
            // Trait+Kx+'a -> Trait+Ky+'b (upcasts).
            (&ty::TyDynamic(ref data_a, r_a), &ty::TyDynamic(ref data_b, r_b)) => {
                // See assemble_candidates_for_unsizing for more info.
                let existential_predicates = data_a.map_bound(|data_a| {
                    let principal = data_a.principal();
                    let iter = principal.into_iter().map(ty::ExistentialPredicate::Trait)
                        .chain(data_a.projection_bounds()
                            .map(|x| ty::ExistentialPredicate::Projection(x)))
                        .chain(data_b.auto_traits().map(ty::ExistentialPredicate::AutoTrait));
                    tcx.mk_existential_predicates(iter)
                });
                let new_trait = tcx.mk_dynamic(existential_predicates, r_b);
                let InferOk { obligations, .. } =
                    self.infcx.at(&obligation.cause, obligation.param_env)
                              .eq(target, new_trait)
                              .map_err(|_| Unimplemented)?;
                nested.extend(obligations);

                // Register one obligation for 'a: 'b.
                let cause = ObligationCause::new(obligation.cause.span,
                                                 obligation.cause.body_id,
                                                 ObjectCastObligation(target));
                let outlives = ty::OutlivesPredicate(r_a, r_b);
                nested.push(Obligation::with_depth(cause,
                                                   obligation.recursion_depth + 1,
                                                   obligation.param_env,
                                                   ty::Binder::bind(outlives).to_predicate()));
            }

            // T -> Trait.
            (_, &ty::TyDynamic(ref data, r)) => {
                let mut object_dids =
                    data.auto_traits().chain(data.principal().map(|p| p.def_id()));
                if let Some(did) = object_dids.find(|did| {
                    !tcx.is_object_safe(*did)
                }) {
                    return Err(TraitNotObjectSafe(did))
                }

                let cause = ObligationCause::new(obligation.cause.span,
                                                 obligation.cause.body_id,
                                                 ObjectCastObligation(target));
                let mut push = |predicate| {
                    nested.push(Obligation::with_depth(cause.clone(),
                                                       obligation.recursion_depth + 1,
                                                       obligation.param_env,
                                                       predicate));
                };

                // Create obligations:
                //  - Casting T to Trait
                //  - For all the various builtin bounds attached to the object cast. (In other
                //  words, if the object type is Foo+Send, this would create an obligation for the
                //  Send check.)
                //  - Projection predicates
                for predicate in data.iter() {
                    push(predicate.with_self_ty(tcx, source));
                }

                // We can only make objects from sized types.
                let tr = ty::TraitRef {
                    def_id: tcx.require_lang_item(lang_items::SizedTraitLangItem),
                    substs: tcx.mk_substs_trait(source, &[]),
                };
                push(tr.to_predicate());

                // If the type is `Foo+'a`, ensures that the type
                // being cast to `Foo+'a` outlives `'a`:
                let outlives = ty::OutlivesPredicate(source, r);
                push(ty::Binder::dummy(outlives).to_predicate());
            }

            // [T; n] -> [T].
            (&ty::TyArray(a, _), &ty::TySlice(b)) => {
                let InferOk { obligations, .. } =
                    self.infcx.at(&obligation.cause, obligation.param_env)
                              .eq(b, a)
                              .map_err(|_| Unimplemented)?;
                nested.extend(obligations);
            }

            // Struct<T> -> Struct<U>.
            (&ty::TyAdt(def, substs_a), &ty::TyAdt(_, substs_b)) => {
                let fields = def
                    .all_fields()
                    .map(|f| tcx.type_of(f.did))
                    .collect::<Vec<_>>();

                // The last field of the structure has to exist and contain type parameters.
                let field = if let Some(&field) = fields.last() {
                    field
                } else {
                    return Err(Unimplemented);
                };
                let mut ty_params = BitVector::new(substs_a.types().count());
                let mut found = false;
                for ty in field.walk() {
                    if let ty::TyParam(p) = ty.sty {
                        ty_params.insert(p.idx as usize);
                        found = true;
                    }
                }
                if !found {
                    return Err(Unimplemented);
                }

                // Replace type parameters used in unsizing with
                // TyError and ensure they do not affect any other fields.
                // This could be checked after type collection for any struct
                // with a potentially unsized trailing field.
                let params = substs_a.iter().enumerate().map(|(i, &k)| {
                    if ty_params.contains(i) {
                        tcx.types.err.into()
                    } else {
                        k
                    }
                });
                let substs = tcx.mk_substs(params);
                for &ty in fields.split_last().unwrap().1 {
                    if ty.subst(tcx, substs).references_error() {
                        return Err(Unimplemented);
                    }
                }

                // Extract Field<T> and Field<U> from Struct<T> and Struct<U>.
                let inner_source = field.subst(tcx, substs_a);
                let inner_target = field.subst(tcx, substs_b);

                // Check that the source struct with the target's
                // unsized parameters is equal to the target.
                let params = substs_a.iter().enumerate().map(|(i, &k)| {
                    if ty_params.contains(i) {
                        substs_b.type_at(i).into()
                    } else {
                        k
                    }
                });
                let new_struct = tcx.mk_adt(def, tcx.mk_substs(params));
                let InferOk { obligations, .. } =
                    self.infcx.at(&obligation.cause, obligation.param_env)
                              .eq(target, new_struct)
                              .map_err(|_| Unimplemented)?;
                nested.extend(obligations);

                // Construct the nested Field<T>: Unsize<Field<U>> predicate.
                nested.push(tcx.predicate_for_trait_def(
                    obligation.param_env,
                    obligation.cause.clone(),
                    obligation.predicate.def_id(),
                    obligation.recursion_depth + 1,
                    inner_source,
                    &[inner_target.into()]));
            }

            // (.., T) -> (.., U).
            (&ty::TyTuple(tys_a), &ty::TyTuple(tys_b)) => {
                assert_eq!(tys_a.len(), tys_b.len());

                // The last field of the tuple has to exist.
                let (&a_last, a_mid) = if let Some(x) = tys_a.split_last() {
                    x
                } else {
                    return Err(Unimplemented);
                };
                let &b_last = tys_b.last().unwrap();

                // Check that the source tuple with the target's
                // last element is equal to the target.
                let new_tuple = tcx.mk_tup(a_mid.iter().cloned().chain(iter::once(b_last)));
                let InferOk { obligations, .. } =
                    self.infcx.at(&obligation.cause, obligation.param_env)
                              .eq(target, new_tuple)
                              .map_err(|_| Unimplemented)?;
                nested.extend(obligations);

                // Construct the nested T: Unsize<U> predicate.
                nested.push(tcx.predicate_for_trait_def(
                    obligation.param_env,
                    obligation.cause.clone(),
                    obligation.predicate.def_id(),
                    obligation.recursion_depth + 1,
                    a_last,
                    &[b_last.into()]));
            }

            _ => bug!()
        };

        Ok(VtableBuiltinData { nested: nested })
    }

    ///////////////////////////////////////////////////////////////////////////
    // Matching
    //
    // Matching is a common path used for both evaluation and
    // confirmation.  It basically unifies types that appear in impls
    // and traits. This does affect the surrounding environment;
    // therefore, when used during evaluation, match routines must be
    // run inside of a `probe()` so that their side-effects are
    // contained.

    fn rematch_impl(&mut self,
                    impl_def_id: DefId,
                    obligation: &TraitObligation<'tcx>,
                    snapshot: &infer::CombinedSnapshot<'cx, 'tcx>)
                    -> (Normalized<'tcx, &'tcx Substs<'tcx>>,
                        infer::SkolemizationMap<'tcx>)
    {
        match self.match_impl(impl_def_id, obligation, snapshot) {
            Ok((substs, skol_map)) => (substs, skol_map),
            Err(()) => {
                bug!("Impl {:?} was matchable against {:?} but now is not",
                     impl_def_id,
                     obligation);
            }
        }
    }

    fn match_impl(&mut self,
                  impl_def_id: DefId,
                  obligation: &TraitObligation<'tcx>,
                  snapshot: &infer::CombinedSnapshot<'cx, 'tcx>)
                  -> Result<(Normalized<'tcx, &'tcx Substs<'tcx>>,
                             infer::SkolemizationMap<'tcx>), ()>
    {
        let impl_trait_ref = self.tcx().impl_trait_ref(impl_def_id).unwrap();

        // Before we create the substitutions and everything, first
        // consider a "quick reject". This avoids creating more types
        // and so forth that we need to.
        if self.fast_reject_trait_refs(obligation, &impl_trait_ref) {
            return Err(());
        }

        let (skol_obligation, skol_map) = self.infcx().skolemize_late_bound_regions(
            &obligation.predicate);
        let skol_obligation_trait_ref = skol_obligation.trait_ref;

        let impl_substs = self.infcx.fresh_substs_for_item(obligation.cause.span,
                                                           impl_def_id);

        let impl_trait_ref = impl_trait_ref.subst(self.tcx(),
                                                  impl_substs);

        let Normalized { value: impl_trait_ref, obligations: mut nested_obligations } =
            project::normalize_with_depth(self,
                                          obligation.param_env,
                                          obligation.cause.clone(),
                                          obligation.recursion_depth + 1,
                                          &impl_trait_ref);

        debug!("match_impl(impl_def_id={:?}, obligation={:?}, \
               impl_trait_ref={:?}, skol_obligation_trait_ref={:?})",
               impl_def_id,
               obligation,
               impl_trait_ref,
               skol_obligation_trait_ref);

        let InferOk { obligations, .. } =
            self.infcx.at(&obligation.cause, obligation.param_env)
                      .eq(skol_obligation_trait_ref, impl_trait_ref)
                      .map_err(|e| {
                          debug!("match_impl: failed eq_trait_refs due to `{}`", e);
                          ()
                      })?;
        nested_obligations.extend(obligations);

        if let Err(e) = self.infcx.leak_check(false,
                                              obligation.cause.span,
                                              &skol_map,
                                              snapshot) {
            debug!("match_impl: failed leak check due to `{}`", e);
            return Err(());
        }

        debug!("match_impl: success impl_substs={:?}", impl_substs);
        Ok((Normalized {
            value: impl_substs,
            obligations: nested_obligations
        }, skol_map))
    }

    fn fast_reject_trait_refs(&mut self,
                              obligation: &TraitObligation,
                              impl_trait_ref: &ty::TraitRef)
                              -> bool
    {
        // We can avoid creating type variables and doing the full
        // substitution if we find that any of the input types, when
        // simplified, do not match.

        obligation.predicate.skip_binder().input_types()
            .zip(impl_trait_ref.input_types())
            .any(|(obligation_ty, impl_ty)| {
                let simplified_obligation_ty =
                    fast_reject::simplify_type(self.tcx(), obligation_ty, true);
                let simplified_impl_ty =
                    fast_reject::simplify_type(self.tcx(), impl_ty, false);

                simplified_obligation_ty.is_some() &&
                    simplified_impl_ty.is_some() &&
                    simplified_obligation_ty != simplified_impl_ty
            })
    }

    /// Normalize `where_clause_trait_ref` and try to match it against
    /// `obligation`.  If successful, return any predicates that
    /// result from the normalization. Normalization is necessary
    /// because where-clauses are stored in the parameter environment
    /// unnormalized.
    fn match_where_clause_trait_ref(&mut self,
                                    obligation: &TraitObligation<'tcx>,
                                    where_clause_trait_ref: ty::PolyTraitRef<'tcx>)
                                    -> Result<Vec<PredicateObligation<'tcx>>,()>
    {
        self.match_poly_trait_ref(obligation, where_clause_trait_ref)
    }

    /// Returns `Ok` if `poly_trait_ref` being true implies that the
    /// obligation is satisfied.
    fn match_poly_trait_ref(&mut self,
                            obligation: &TraitObligation<'tcx>,
                            poly_trait_ref: ty::PolyTraitRef<'tcx>)
                            -> Result<Vec<PredicateObligation<'tcx>>,()>
    {
        debug!("match_poly_trait_ref: obligation={:?} poly_trait_ref={:?}",
               obligation,
               poly_trait_ref);

        self.infcx.at(&obligation.cause, obligation.param_env)
                  .sup(obligation.predicate.to_poly_trait_ref(), poly_trait_ref)
                  .map(|InferOk { obligations, .. }| obligations)
                  .map_err(|_| ())
    }

    ///////////////////////////////////////////////////////////////////////////
    // Miscellany

    fn match_fresh_trait_refs(&self,
                              previous: &ty::PolyTraitRef<'tcx>,
                              current: &ty::PolyTraitRef<'tcx>)
                              -> bool
    {
        let mut matcher = ty::_match::Match::new(self.tcx());
        matcher.relate(previous, current).is_ok()
    }

    fn push_stack<'o,'s:'o>(&mut self,
                            previous_stack: TraitObligationStackList<'s, 'tcx>,
                            obligation: &'o TraitObligation<'tcx>)
                            -> TraitObligationStack<'o, 'tcx>
    {
        let fresh_trait_ref =
            obligation.predicate.to_poly_trait_ref().fold_with(&mut self.freshener);

        TraitObligationStack {
            obligation,
            fresh_trait_ref,
            previous: previous_stack,
        }
    }

    fn closure_trait_ref_unnormalized(&mut self,
                                      obligation: &TraitObligation<'tcx>,
                                      closure_def_id: DefId,
                                      substs: ty::ClosureSubsts<'tcx>)
                                      -> ty::PolyTraitRef<'tcx>
    {
        let closure_type = self.infcx.closure_sig(closure_def_id, substs);

        // (1) Feels icky to skip the binder here, but OTOH we know
        // that the self-type is an unboxed closure type and hence is
        // in fact unparameterized (or at least does not reference any
        // regions bound in the obligation). Still probably some
        // refactoring could make this nicer.

        self.tcx().closure_trait_ref_and_return_type(obligation.predicate.def_id(),
                                                     obligation.predicate
                                                         .skip_binder().self_ty(), // (1)
                                                     closure_type,
                                                     util::TupleArgumentsFlag::No)
            .map_bound(|(trait_ref, _)| trait_ref)
    }

    fn generator_trait_ref_unnormalized(&mut self,
                                      obligation: &TraitObligation<'tcx>,
                                      closure_def_id: DefId,
                                      substs: ty::GeneratorSubsts<'tcx>)
                                      -> ty::PolyTraitRef<'tcx>
    {
        let gen_sig = substs.poly_sig(closure_def_id, self.tcx());

        // (1) Feels icky to skip the binder here, but OTOH we know
        // that the self-type is an generator type and hence is
        // in fact unparameterized (or at least does not reference any
        // regions bound in the obligation). Still probably some
        // refactoring could make this nicer.

        self.tcx().generator_trait_ref_and_outputs(obligation.predicate.def_id(),
                                                   obligation.predicate
                                                       .skip_binder().self_ty(), // (1)
                                                   gen_sig)
            .map_bound(|(trait_ref, ..)| trait_ref)
    }

    /// Returns the obligations that are implied by instantiating an
    /// impl or trait. The obligations are substituted and fully
    /// normalized. This is used when confirming an impl or default
    /// impl.
    fn impl_or_trait_obligations(&mut self,
                                 cause: ObligationCause<'tcx>,
                                 recursion_depth: usize,
                                 param_env: ty::ParamEnv<'tcx>,
                                 def_id: DefId, // of impl or trait
                                 substs: &Substs<'tcx>, // for impl or trait
                                 skol_map: infer::SkolemizationMap<'tcx>,
                                 snapshot: &infer::CombinedSnapshot<'cx, 'tcx>)
                                 -> Vec<PredicateObligation<'tcx>>
    {
        debug!("impl_or_trait_obligations(def_id={:?})", def_id);
        let tcx = self.tcx();

        // To allow for one-pass evaluation of the nested obligation,
        // each predicate must be preceded by the obligations required
        // to normalize it.
        // for example, if we have:
        //    impl<U: Iterator, V: Iterator<Item=U>> Foo for V where U::Item: Copy
        // the impl will have the following predicates:
        //    <V as Iterator>::Item = U,
        //    U: Iterator, U: Sized,
        //    V: Iterator, V: Sized,
        //    <U as Iterator>::Item: Copy
        // When we substitute, say, `V => IntoIter<u32>, U => $0`, the last
        // obligation will normalize to `<$0 as Iterator>::Item = $1` and
        // `$1: Copy`, so we must ensure the obligations are emitted in
        // that order.
        let predicates = tcx.predicates_of(def_id);
        assert_eq!(predicates.parent, None);
        let mut predicates: Vec<_> = predicates.predicates.iter().flat_map(|predicate| {
            let predicate = normalize_with_depth(self, param_env, cause.clone(), recursion_depth,
                                                 &predicate.subst(tcx, substs));
            predicate.obligations.into_iter().chain(
                Some(Obligation {
                    cause: cause.clone(),
                    recursion_depth,
                    param_env,
                    predicate: predicate.value
                }))
        }).collect();

        // We are performing deduplication here to avoid exponential blowups
        // (#38528) from happening, but the real cause of the duplication is
        // unknown. What we know is that the deduplication avoids exponential
        // amount of predicates being propagated when processing deeply nested
        // types.
        //
        // This code is hot enough that it's worth avoiding the allocation
        // required for the FxHashSet when possible. Special-casing lengths 0,
        // 1 and 2 covers roughly 75--80% of the cases.
        if predicates.len() <= 1 {
            // No possibility of duplicates.
        } else if predicates.len() == 2 {
            // Only two elements. Drop the second if they are equal.
            if predicates[0] == predicates[1] {
                predicates.truncate(1);
            }
        } else {
            // Three or more elements. Use a general deduplication process.
            let mut seen = FxHashSet();
            predicates.retain(|i| seen.insert(i.clone()));
        }
        self.infcx().plug_leaks(skol_map, snapshot, predicates)
    }
}

impl<'tcx> TraitObligation<'tcx> {
    #[allow(unused_comparisons)]
    pub fn derived_cause(&self,
                        variant: fn(DerivedObligationCause<'tcx>) -> ObligationCauseCode<'tcx>)
                        -> ObligationCause<'tcx>
    {
        /*!
         * Creates a cause for obligations that are derived from
         * `obligation` by a recursive search (e.g., for a builtin
         * bound, or eventually a `auto trait Foo`). If `obligation`
         * is itself a derived obligation, this is just a clone, but
         * otherwise we create a "derived obligation" cause so as to
         * keep track of the original root obligation for error
         * reporting.
         */

        let obligation = self;

        // NOTE(flaper87): As of now, it keeps track of the whole error
        // chain. Ideally, we should have a way to configure this either
        // by using -Z verbose or just a CLI argument.
        if obligation.recursion_depth >= 0 {
            let derived_cause = DerivedObligationCause {
                parent_trait_ref: obligation.predicate.to_poly_trait_ref(),
                parent_code: Rc::new(obligation.cause.code.clone())
            };
            let derived_code = variant(derived_cause);
            ObligationCause::new(obligation.cause.span, obligation.cause.body_id, derived_code)
        } else {
            obligation.cause.clone()
        }
    }
}

impl<'tcx> SelectionCache<'tcx> {
    pub fn new() -> SelectionCache<'tcx> {
        SelectionCache {
            hashmap: Lock::new(FxHashMap())
        }
    }

    pub fn clear(&self) {
        *self.hashmap.borrow_mut() = FxHashMap()
    }
}

impl<'tcx> EvaluationCache<'tcx> {
    pub fn new() -> EvaluationCache<'tcx> {
        EvaluationCache {
            hashmap: Lock::new(FxHashMap())
        }
    }

    pub fn clear(&self) {
        *self.hashmap.borrow_mut() = FxHashMap()
    }
}

impl<'o,'tcx> TraitObligationStack<'o,'tcx> {
    fn list(&'o self) -> TraitObligationStackList<'o,'tcx> {
        TraitObligationStackList::with(self)
    }

    fn iter(&'o self) -> TraitObligationStackList<'o,'tcx> {
        self.list()
    }
}

#[derive(Copy, Clone)]
struct TraitObligationStackList<'o,'tcx:'o> {
    head: Option<&'o TraitObligationStack<'o,'tcx>>
}

impl<'o,'tcx> TraitObligationStackList<'o,'tcx> {
    fn empty() -> TraitObligationStackList<'o,'tcx> {
        TraitObligationStackList { head: None }
    }

    fn with(r: &'o TraitObligationStack<'o,'tcx>) -> TraitObligationStackList<'o,'tcx> {
        TraitObligationStackList { head: Some(r) }
    }
}

impl<'o,'tcx> Iterator for TraitObligationStackList<'o,'tcx>{
    type Item = &'o TraitObligationStack<'o,'tcx>;

    fn next(&mut self) -> Option<&'o TraitObligationStack<'o,'tcx>> {
        match self.head {
            Some(o) => {
                *self = o.previous;
                Some(o)
            }
            None => None
        }
    }
}

impl<'o,'tcx> fmt::Debug for TraitObligationStack<'o,'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TraitObligationStack({:?})", self.obligation)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct WithDepNode<T> {
    dep_node: DepNodeIndex,
    cached_value: T
}

impl<T: Clone> WithDepNode<T> {
    pub fn new(dep_node: DepNodeIndex, cached_value: T) -> Self {
        WithDepNode { dep_node, cached_value }
    }

    pub fn get(&self, tcx: TyCtxt) -> T {
        tcx.dep_graph.read_index(self.dep_node);
        self.cached_value.clone()
    }
}
