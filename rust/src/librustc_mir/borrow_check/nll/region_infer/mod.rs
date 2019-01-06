use super::universal_regions::UniversalRegions;
use borrow_check::nll::constraints::graph::NormalConstraintGraph;
use borrow_check::nll::constraints::{ConstraintSccIndex, ConstraintSet, OutlivesConstraint};
use borrow_check::nll::region_infer::values::{PlaceholderIndices, RegionElement, ToElementIndex};
use borrow_check::nll::type_check::free_region_relations::UniversalRegionRelations;
use borrow_check::nll::type_check::Locations;
use rustc::hir::def_id::DefId;
use rustc::infer::canonical::QueryRegionConstraint;
use rustc::infer::region_constraints::{GenericKind, VarInfos, VerifyBound};
use rustc::infer::{InferCtxt, NLLRegionVariableOrigin, RegionVariableOrigin};
use rustc::mir::{
    ClosureOutlivesRequirement, ClosureOutlivesSubject, ClosureRegionRequirements,
    ConstraintCategory, Local, Location, Mir,
};
use rustc::ty::{self, RegionVid, Ty, TyCtxt, TypeFoldable};
use rustc::util::common;
use rustc_data_structures::bit_set::BitSet;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::graph::scc::Sccs;
use rustc_data_structures::indexed_vec::IndexVec;
use rustc_errors::{Diagnostic, DiagnosticBuilder};
use syntax_pos::Span;

use std::rc::Rc;

mod dump_mir;
mod error_reporting;
crate use self::error_reporting::{RegionName, RegionNameSource};
mod graphviz;
pub mod values;
use self::values::{LivenessValues, RegionValueElements, RegionValues};

use super::ToRegionVid;

pub struct RegionInferenceContext<'tcx> {
    /// Contains the definition for every region variable.  Region
    /// variables are identified by their index (`RegionVid`). The
    /// definition contains information about where the region came
    /// from as well as its final inferred value.
    definitions: IndexVec<RegionVid, RegionDefinition<'tcx>>,

    /// The liveness constraints added to each region. For most
    /// regions, these start out empty and steadily grow, though for
    /// each universally quantified region R they start out containing
    /// the entire CFG and `end(R)`.
    liveness_constraints: LivenessValues<RegionVid>,

    /// The outlives constraints computed by the type-check.
    constraints: Rc<ConstraintSet>,

    /// The constraint-set, but in graph form, making it easy to traverse
    /// the constraints adjacent to a particular region. Used to construct
    /// the SCC (see `constraint_sccs`) and for error reporting.
    constraint_graph: Rc<NormalConstraintGraph>,

    /// The SCC computed from `constraints` and the constraint graph. Used to
    /// compute the values of each region.
    constraint_sccs: Rc<Sccs<RegionVid, ConstraintSccIndex>>,

    /// Map closure bounds to a `Span` that should be used for error reporting.
    closure_bounds_mapping:
        FxHashMap<Location, FxHashMap<(RegionVid, RegionVid), (ConstraintCategory, Span)>>,

    /// Contains the minimum universe of any variable within the same
    /// SCC. We will ensure that no SCC contains values that are not
    /// visible from this index.
    scc_universes: IndexVec<ConstraintSccIndex, ty::UniverseIndex>,

    /// Contains a "representative" from each SCC. This will be the
    /// minimal RegionVid belonging to that universe. It is used as a
    /// kind of hacky way to manage checking outlives relationships,
    /// since we can 'canonicalize' each region to the representative
    /// of its SCC and be sure that -- if they have the same repr --
    /// they *must* be equal (though not having the same repr does not
    /// mean they are unequal).
    scc_representatives: IndexVec<ConstraintSccIndex, ty::RegionVid>,

    /// The final inferred values of the region variables; we compute
    /// one value per SCC. To get the value for any given *region*,
    /// you first find which scc it is a part of.
    scc_values: RegionValues<ConstraintSccIndex>,

    /// Type constraints that we check after solving.
    type_tests: Vec<TypeTest<'tcx>>,

    /// Information about the universally quantified regions in scope
    /// on this function.
    universal_regions: Rc<UniversalRegions<'tcx>>,

    /// Information about how the universally quantified regions in
    /// scope on this function relate to one another.
    universal_region_relations: Rc<UniversalRegionRelations<'tcx>>,
}

struct RegionDefinition<'tcx> {
    /// What kind of variable is this -- a free region? existential
    /// variable? etc. (See the `NLLRegionVariableOrigin` for more
    /// info.)
    origin: NLLRegionVariableOrigin,

    /// Which universe is this region variable defined in? This is
    /// most often `ty::UniverseIndex::ROOT`, but when we encounter
    /// forall-quantifiers like `for<'a> { 'a = 'b }`, we would create
    /// the variable for `'a` in a fresh universe that extends ROOT.
    universe: ty::UniverseIndex,

    /// If this is 'static or an early-bound region, then this is
    /// `Some(X)` where `X` is the name of the region.
    external_name: Option<ty::Region<'tcx>>,
}

/// N.B., the variants in `Cause` are intentionally ordered. Lower
/// values are preferred when it comes to error messages. Do not
/// reorder willy nilly.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) enum Cause {
    /// point inserted because Local was live at the given Location
    LiveVar(Local, Location),

    /// point inserted because Local was dropped at the given Location
    DropVar(Local, Location),
}

/// A "type test" corresponds to an outlives constraint between a type
/// and a lifetime, like `T: 'x` or `<T as Foo>::Bar: 'x`.  They are
/// translated from the `Verify` region constraints in the ordinary
/// inference context.
///
/// These sorts of constraints are handled differently than ordinary
/// constraints, at least at present. During type checking, the
/// `InferCtxt::process_registered_region_obligations` method will
/// attempt to convert a type test like `T: 'x` into an ordinary
/// outlives constraint when possible (for example, `&'a T: 'b` will
/// be converted into `'a: 'b` and registered as a `Constraint`).
///
/// In some cases, however, there are outlives relationships that are
/// not converted into a region constraint, but rather into one of
/// these "type tests".  The distinction is that a type test does not
/// influence the inference result, but instead just examines the
/// values that we ultimately inferred for each region variable and
/// checks that they meet certain extra criteria.  If not, an error
/// can be issued.
///
/// One reason for this is that these type tests typically boil down
/// to a check like `'a: 'x` where `'a` is a universally quantified
/// region -- and therefore not one whose value is really meant to be
/// *inferred*, precisely (this is not always the case: one can have a
/// type test like `<Foo as Trait<'?0>>::Bar: 'x`, where `'?0` is an
/// inference variable). Another reason is that these type tests can
/// involve *disjunction* -- that is, they can be satisfied in more
/// than one way.
///
/// For more information about this translation, see
/// `InferCtxt::process_registered_region_obligations` and
/// `InferCtxt::type_must_outlive` in `rustc::infer::outlives`.
#[derive(Clone, Debug)]
pub struct TypeTest<'tcx> {
    /// The type `T` that must outlive the region.
    pub generic_kind: GenericKind<'tcx>,

    /// The region `'x` that the type must outlive.
    pub lower_bound: RegionVid,

    /// Where did this constraint arise and why?
    pub locations: Locations,

    /// A test which, if met by the region `'x`, proves that this type
    /// constraint is satisfied.
    pub verify_bound: VerifyBound<'tcx>,
}

impl<'tcx> RegionInferenceContext<'tcx> {
    /// Creates a new region inference context with a total of
    /// `num_region_variables` valid inference variables; the first N
    /// of those will be constant regions representing the free
    /// regions defined in `universal_regions`.
    ///
    /// The `outlives_constraints` and `type_tests` are an initial set
    /// of constraints produced by the MIR type check.
    pub(crate) fn new(
        var_infos: VarInfos,
        universal_regions: Rc<UniversalRegions<'tcx>>,
        placeholder_indices: Rc<PlaceholderIndices>,
        universal_region_relations: Rc<UniversalRegionRelations<'tcx>>,
        _mir: &Mir<'tcx>,
        outlives_constraints: ConstraintSet,
        closure_bounds_mapping: FxHashMap<
            Location,
            FxHashMap<(RegionVid, RegionVid), (ConstraintCategory, Span)>,
        >,
        type_tests: Vec<TypeTest<'tcx>>,
        liveness_constraints: LivenessValues<RegionVid>,
        elements: &Rc<RegionValueElements>,
    ) -> Self {
        // Create a RegionDefinition for each inference variable.
        let definitions: IndexVec<_, _> = var_infos
            .into_iter()
            .map(|info| RegionDefinition::new(info.universe, info.origin))
            .collect();

        let constraints = Rc::new(outlives_constraints); // freeze constraints
        let constraint_graph = Rc::new(constraints.graph(definitions.len()));
        let fr_static = universal_regions.fr_static;
        let constraint_sccs = Rc::new(constraints.compute_sccs(&constraint_graph, fr_static));

        let mut scc_values =
            RegionValues::new(elements, universal_regions.len(), &placeholder_indices);

        for region in liveness_constraints.rows() {
            let scc = constraint_sccs.scc(region);
            scc_values.merge_liveness(scc, region, &liveness_constraints);
        }

        let scc_universes = Self::compute_scc_universes(&constraint_sccs, &definitions);

        let scc_representatives = Self::compute_scc_representatives(&constraint_sccs, &definitions);

        let mut result = Self {
            definitions,
            liveness_constraints,
            constraints,
            constraint_graph,
            constraint_sccs,
            closure_bounds_mapping,
            scc_universes,
            scc_representatives,
            scc_values,
            type_tests,
            universal_regions,
            universal_region_relations,
        };

        result.init_free_and_bound_regions();

        result
    }

    /// Each SCC is the combination of many region variables which
    /// have been equated. Therefore, we can associate a universe with
    /// each SCC which is minimum of all the universes of its
    /// constituent regions -- this is because whatever value the SCC
    /// takes on must be a value that each of the regions within the
    /// SCC could have as well. This implies that the SCC must have
    /// the minimum, or narrowest, universe.
    fn compute_scc_universes(
        constraints_scc: &Sccs<RegionVid, ConstraintSccIndex>,
        definitions: &IndexVec<RegionVid, RegionDefinition<'tcx>>,
    ) -> IndexVec<ConstraintSccIndex, ty::UniverseIndex> {
        let num_sccs = constraints_scc.num_sccs();
        let mut scc_universes = IndexVec::from_elem_n(ty::UniverseIndex::MAX, num_sccs);

        for (region_vid, region_definition) in definitions.iter_enumerated() {
            let scc = constraints_scc.scc(region_vid);
            let scc_universe = &mut scc_universes[scc];
            *scc_universe = ::std::cmp::min(*scc_universe, region_definition.universe);
        }

        debug!("compute_scc_universes: scc_universe = {:#?}", scc_universes);

        scc_universes
    }

    /// For each SCC, we compute a unique `RegionVid` (in fact, the
    /// minimal one that belongs to the SCC). See
    /// `scc_representatives` field of `RegionInferenceContext` for
    /// more details.
    fn compute_scc_representatives(
        constraints_scc: &Sccs<RegionVid, ConstraintSccIndex>,
        definitions: &IndexVec<RegionVid, RegionDefinition<'tcx>>,
    ) -> IndexVec<ConstraintSccIndex, ty::RegionVid> {
        let num_sccs = constraints_scc.num_sccs();
        let next_region_vid = definitions.next_index();
        let mut scc_representatives = IndexVec::from_elem_n(next_region_vid, num_sccs);

        for region_vid in definitions.indices() {
            let scc = constraints_scc.scc(region_vid);
            let prev_min = scc_representatives[scc];
            scc_representatives[scc] = region_vid.min(prev_min);
        }

        scc_representatives
    }

    /// Initializes the region variables for each universally
    /// quantified region (lifetime parameter). The first N variables
    /// always correspond to the regions appearing in the function
    /// signature (both named and anonymous) and where clauses. This
    /// function iterates over those regions and initializes them with
    /// minimum values.
    ///
    /// For example:
    ///
    ///     fn foo<'a, 'b>(..) where 'a: 'b
    ///
    /// would initialize two variables like so:
    ///
    ///     R0 = { CFG, R0 } // 'a
    ///     R1 = { CFG, R0, R1 } // 'b
    ///
    /// Here, R0 represents `'a`, and it contains (a) the entire CFG
    /// and (b) any universally quantified regions that it outlives,
    /// which in this case is just itself. R1 (`'b`) in contrast also
    /// outlives `'a` and hence contains R0 and R1.
    fn init_free_and_bound_regions(&mut self) {
        // Update the names (if any)
        for (external_name, variable) in self.universal_regions.named_universal_regions() {
            debug!(
                "init_universal_regions: region {:?} has external name {:?}",
                variable, external_name
            );
            self.definitions[variable].external_name = Some(external_name);
        }

        for variable in self.definitions.indices() {
            let scc = self.constraint_sccs.scc(variable);

            match self.definitions[variable].origin {
                NLLRegionVariableOrigin::FreeRegion => {
                    // For each free, universally quantified region X:

                    // Add all nodes in the CFG to liveness constraints
                    self.liveness_constraints.add_all_points(variable);
                    self.scc_values.add_all_points(scc);

                    // Add `end(X)` into the set for X.
                    self.scc_values.add_element(scc, variable);
                }

                NLLRegionVariableOrigin::Placeholder(placeholder) => {
                    // Each placeholder region is only visible from
                    // its universe `ui` and its extensions. So we
                    // can't just add it into `scc` unless the
                    // universe of the scc can name this region.
                    let scc_universe = self.scc_universes[scc];
                    if scc_universe.can_name(placeholder.universe) {
                        self.scc_values.add_element(scc, placeholder);
                    } else {
                        debug!(
                            "init_free_and_bound_regions: placeholder {:?} is \
                             not compatible with universe {:?} of its SCC {:?}",
                            placeholder,
                            scc_universe,
                            scc,
                        );
                        self.add_incompatible_universe(scc);
                    }
                }

                NLLRegionVariableOrigin::Existential => {
                    // For existential, regions, nothing to do.
                }
            }
        }
    }

    /// Returns an iterator over all the region indices.
    pub fn regions(&self) -> impl Iterator<Item = RegionVid> {
        self.definitions.indices()
    }

    /// Given a universal region in scope on the MIR, returns the
    /// corresponding index.
    ///
    /// (Panics if `r` is not a registered universal region.)
    pub fn to_region_vid(&self, r: ty::Region<'tcx>) -> RegionVid {
        self.universal_regions.to_region_vid(r)
    }

    /// Add annotations for `#[rustc_regions]`; see `UniversalRegions::annotate`.
    crate fn annotate(&self, tcx: TyCtxt<'_, '_, 'tcx>, err: &mut DiagnosticBuilder<'_>) {
        self.universal_regions.annotate(tcx, err)
    }

    /// Returns true if the region `r` contains the point `p`.
    ///
    /// Panics if called before `solve()` executes,
    crate fn region_contains(&self, r: impl ToRegionVid, p: impl ToElementIndex) -> bool {
        let scc = self.constraint_sccs.scc(r.to_region_vid());
        self.scc_values.contains(scc, p)
    }

    /// Returns access to the value of `r` for debugging purposes.
    crate fn region_value_str(&self, r: RegionVid) -> String {
        let scc = self.constraint_sccs.scc(r.to_region_vid());
        self.scc_values.region_value_str(scc)
    }

    /// Returns access to the value of `r` for debugging purposes.
    crate fn region_universe(&self, r: RegionVid) -> ty::UniverseIndex {
        let scc = self.constraint_sccs.scc(r.to_region_vid());
        self.scc_universes[scc]
    }

    /// Perform region inference and report errors if we see any
    /// unsatisfiable constraints. If this is a closure, returns the
    /// region requirements to propagate to our creator, if any.
    pub(super) fn solve<'gcx>(
        &mut self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        mir_def_id: DefId,
        errors_buffer: &mut Vec<Diagnostic>,
    ) -> Option<ClosureRegionRequirements<'gcx>> {
        common::time(
            infcx.tcx.sess,
            &format!("solve_nll_region_constraints({:?})", mir_def_id),
            || self.solve_inner(infcx, mir, mir_def_id, errors_buffer),
        )
    }

    fn solve_inner<'gcx>(
        &mut self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        mir_def_id: DefId,
        errors_buffer: &mut Vec<Diagnostic>,
    ) -> Option<ClosureRegionRequirements<'gcx>> {
        self.propagate_constraints(mir);

        // If this is a closure, we can propagate unsatisfied
        // `outlives_requirements` to our creator, so create a vector
        // to store those. Otherwise, we'll pass in `None` to the
        // functions below, which will trigger them to report errors
        // eagerly.
        let mut outlives_requirements = if infcx.tcx.is_closure(mir_def_id) {
            Some(vec![])
        } else {
            None
        };

        self.check_type_tests(
            infcx,
            mir,
            mir_def_id,
            outlives_requirements.as_mut(),
            errors_buffer,
        );

        self.check_universal_regions(
            infcx,
            mir,
            mir_def_id,
            outlives_requirements.as_mut(),
            errors_buffer,
        );

        let outlives_requirements = outlives_requirements.unwrap_or(vec![]);

        if outlives_requirements.is_empty() {
            None
        } else {
            let num_external_vids = self.universal_regions.num_global_and_external_regions();
            Some(ClosureRegionRequirements {
                num_external_vids,
                outlives_requirements,
            })
        }
    }

    /// Propagate the region constraints: this will grow the values
    /// for each region variable until all the constraints are
    /// satisfied. Note that some values may grow **too** large to be
    /// feasible, but we check this later.
    fn propagate_constraints(&mut self, _mir: &Mir<'tcx>) {
        debug!("propagate_constraints()");

        debug!("propagate_constraints: constraints={:#?}", {
            let mut constraints: Vec<_> = self.constraints.iter().collect();
            constraints.sort();
            constraints
                .into_iter()
                .map(|c| (c, self.constraint_sccs.scc(c.sup), self.constraint_sccs.scc(c.sub)))
                .collect::<Vec<_>>()
        });

        // To propagate constraints, we walk the DAG induced by the
        // SCC. For each SCC, we visit its successors and compute
        // their values, then we union all those values to get our
        // own.
        let visited = &mut BitSet::new_empty(self.constraint_sccs.num_sccs());
        for scc_index in self.constraint_sccs.all_sccs() {
            self.propagate_constraint_sccs_if_new(scc_index, visited);
        }
    }

    #[inline]
    fn propagate_constraint_sccs_if_new(
        &mut self,
        scc_a: ConstraintSccIndex,
        visited: &mut BitSet<ConstraintSccIndex>,
    ) {
        if visited.insert(scc_a) {
            self.propagate_constraint_sccs_new(scc_a, visited);
        }
    }

    fn propagate_constraint_sccs_new(
        &mut self,
        scc_a: ConstraintSccIndex,
        visited: &mut BitSet<ConstraintSccIndex>,
    ) {
        let constraint_sccs = self.constraint_sccs.clone();

        // Walk each SCC `B` such that `A: B`...
        for &scc_b in constraint_sccs.successors(scc_a) {
            debug!(
                "propagate_constraint_sccs: scc_a = {:?} scc_b = {:?}",
                scc_a, scc_b
            );

            // ...compute the value of `B`...
            self.propagate_constraint_sccs_if_new(scc_b, visited);

            // ...and add elements from `B` into `A`. One complication
            // arises because of universes: If `B` contains something
            // that `A` cannot name, then `A` can only contain `B` if
            // it outlives static.
            if self.universe_compatible(scc_b, scc_a) {
                // `A` can name everything that is in `B`, so just
                // merge the bits.
                self.scc_values.add_region(scc_a, scc_b);
            } else {
                self.add_incompatible_universe(scc_a);
            }
        }

        debug!(
            "propagate_constraint_sccs: scc_a = {:?} has value {:?}",
            scc_a,
            self.scc_values.region_value_str(scc_a),
        );
    }

    /// True if all the elements in the value of `scc_b` are nameable
    /// in `scc_a`. Used during constraint propagation, and only once
    /// the value of `scc_b` has been computed.
    fn universe_compatible(&self, scc_b: ConstraintSccIndex, scc_a: ConstraintSccIndex) -> bool {
        let universe_a = self.scc_universes[scc_a];

        // Quick check: if scc_b's declared universe is a subset of
        // scc_a's declared univese (typically, both are ROOT), then
        // it cannot contain any problematic universe elements.
        if universe_a.can_name(self.scc_universes[scc_b]) {
            return true;
        }

        // Otherwise, we have to iterate over the universe elements in
        // B's value, and check whether all of them are nameable
        // from universe_a
        self.scc_values
            .placeholders_contained_in(scc_b)
            .all(|p| universe_a.can_name(p.universe))
    }

    /// Extend `scc` so that it can outlive some placeholder region
    /// from a universe it can't name; at present, the only way for
    /// this to be true is if `scc` outlives `'static`. This is
    /// actually stricter than necessary: ideally, we'd support bounds
    /// like `for<'a: 'b`>` that might then allow us to approximate
    /// `'a` with `'b` and not `'static`. But it will have to do for
    /// now.
    fn add_incompatible_universe(&mut self, scc: ConstraintSccIndex) {
        debug!("add_incompatible_universe(scc={:?})", scc);

        let fr_static = self.universal_regions.fr_static;
        self.scc_values.add_all_points(scc);
        self.scc_values.add_element(scc, fr_static);
    }

    /// Once regions have been propagated, this method is used to see
    /// whether the "type tests" produced by typeck were satisfied;
    /// type tests encode type-outlives relationships like `T:
    /// 'a`. See `TypeTest` for more details.
    fn check_type_tests<'gcx>(
        &self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        mir_def_id: DefId,
        mut propagated_outlives_requirements: Option<&mut Vec<ClosureOutlivesRequirement<'gcx>>>,
        errors_buffer: &mut Vec<Diagnostic>,
    ) {
        let tcx = infcx.tcx;

        // Sometimes we register equivalent type-tests that would
        // result in basically the exact same error being reported to
        // the user. Avoid that.
        let mut deduplicate_errors = FxHashSet::default();

        for type_test in &self.type_tests {
            debug!("check_type_test: {:?}", type_test);

            let generic_ty = type_test.generic_kind.to_ty(tcx);
            if self.eval_verify_bound(
                tcx,
                mir,
                generic_ty,
                type_test.lower_bound,
                &type_test.verify_bound,
            ) {
                continue;
            }

            if let Some(propagated_outlives_requirements) = &mut propagated_outlives_requirements {
                if self.try_promote_type_test(
                    infcx,
                    mir,
                    type_test,
                    propagated_outlives_requirements,
                ) {
                    continue;
                }
            }

            // Type-test failed. Report the error.

            // Try to convert the lower-bound region into something named we can print for the user.
            let lower_bound_region = self.to_error_region(type_test.lower_bound);

            // Skip duplicate-ish errors.
            let type_test_span = type_test.locations.span(mir);
            let erased_generic_kind = tcx.erase_regions(&type_test.generic_kind);
            if !deduplicate_errors.insert((
                erased_generic_kind,
                lower_bound_region,
                type_test.locations,
            )) {
                continue;
            } else {
                debug!(
                    "check_type_test: reporting error for erased_generic_kind={:?}, \
                     lower_bound_region={:?}, \
                     type_test.locations={:?}",
                    erased_generic_kind, lower_bound_region, type_test.locations,
                );
            }

            if let Some(lower_bound_region) = lower_bound_region {
                let region_scope_tree = &tcx.region_scope_tree(mir_def_id);
                infcx
                    .construct_generic_bound_failure(
                        region_scope_tree,
                        type_test_span,
                        None,
                        type_test.generic_kind,
                        lower_bound_region,
                    )
                    .buffer(errors_buffer);
            } else {
                // FIXME. We should handle this case better. It
                // indicates that we have e.g., some region variable
                // whose value is like `'a+'b` where `'a` and `'b` are
                // distinct unrelated univesal regions that are not
                // known to outlive one another. It'd be nice to have
                // some examples where this arises to decide how best
                // to report it; we could probably handle it by
                // iterating over the universal regions and reporting
                // an error that multiple bounds are required.
                tcx.sess
                    .struct_span_err(
                        type_test_span,
                        &format!("`{}` does not live long enough", type_test.generic_kind,),
                    )
                    .buffer(errors_buffer);
            }
        }
    }

    /// Converts a region inference variable into a `ty::Region` that
    /// we can use for error reporting. If `r` is universally bound,
    /// then we use the name that we have on record for it. If `r` is
    /// existentially bound, then we check its inferred value and try
    /// to find a good name from that. Returns `None` if we can't find
    /// one (e.g., this is just some random part of the CFG).
    pub fn to_error_region(&self, r: RegionVid) -> Option<ty::Region<'tcx>> {
        self.to_error_region_vid(r).and_then(|r| self.definitions[r].external_name)
    }

    /// Returns the [RegionVid] corresponding to the region returned by
    /// `to_error_region`.
    pub fn to_error_region_vid(&self, r: RegionVid) -> Option<RegionVid> {
        if self.universal_regions.is_universal_region(r) {
            Some(r)
        } else {
            let r_scc = self.constraint_sccs.scc(r);
            let upper_bound = self.universal_upper_bound(r);
            if self.scc_values.contains(r_scc, upper_bound) {
                self.to_error_region_vid(upper_bound)
            } else {
                None
            }
        }
    }

    /// Invoked when we have some type-test (e.g., `T: 'X`) that we cannot
    /// prove to be satisfied. If this is a closure, we will attempt to
    /// "promote" this type-test into our `ClosureRegionRequirements` and
    /// hence pass it up the creator. To do this, we have to phrase the
    /// type-test in terms of external free regions, as local free
    /// regions are not nameable by the closure's creator.
    ///
    /// Promotion works as follows: we first check that the type `T`
    /// contains only regions that the creator knows about. If this is
    /// true, then -- as a consequence -- we know that all regions in
    /// the type `T` are free regions that outlive the closure body. If
    /// false, then promotion fails.
    ///
    /// Once we've promoted T, we have to "promote" `'X` to some region
    /// that is "external" to the closure. Generally speaking, a region
    /// may be the union of some points in the closure body as well as
    /// various free lifetimes. We can ignore the points in the closure
    /// body: if the type T can be expressed in terms of external regions,
    /// we know it outlives the points in the closure body. That
    /// just leaves the free regions.
    ///
    /// The idea then is to lower the `T: 'X` constraint into multiple
    /// bounds -- e.g., if `'X` is the union of two free lifetimes,
    /// `'1` and `'2`, then we would create `T: '1` and `T: '2`.
    fn try_promote_type_test<'gcx>(
        &self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        type_test: &TypeTest<'tcx>,
        propagated_outlives_requirements: &mut Vec<ClosureOutlivesRequirement<'gcx>>,
    ) -> bool {
        let tcx = infcx.tcx;

        let TypeTest {
            generic_kind,
            lower_bound,
            locations,
            verify_bound: _,
        } = type_test;

        let generic_ty = generic_kind.to_ty(tcx);
        let subject = match self.try_promote_type_test_subject(infcx, generic_ty) {
            Some(s) => s,
            None => return false,
        };

        // For each region outlived by lower_bound find a non-local,
        // universal region (it may be the same region) and add it to
        // `ClosureOutlivesRequirement`.
        let r_scc = self.constraint_sccs.scc(*lower_bound);
        for ur in self.scc_values.universal_regions_outlived_by(r_scc) {
            // Check whether we can already prove that the "subject" outlives `ur`.
            // If so, we don't have to propagate this requirement to our caller.
            //
            // To continue the example from the function, if we are trying to promote
            // a requirement that `T: 'X`, and we know that `'X = '1 + '2` (i.e., the union
            // `'1` and `'2`), then in this loop `ur` will be `'1` (and `'2`). So here
            // we check whether `T: '1` is something we *can* prove. If so, no need
            // to propagate that requirement.
            //
            // This is needed because -- particularly in the case
            // where `ur` is a local bound -- we are sometimes in a
            // position to prove things that our caller cannot.  See
            // #53570 for an example.
            if self.eval_verify_bound(tcx, mir, generic_ty, ur, &type_test.verify_bound) {
                continue;
            }

            debug!("try_promote_type_test: ur={:?}", ur);

            let non_local_ub = self.universal_region_relations.non_local_upper_bound(ur);
            debug!("try_promote_type_test: non_local_ub={:?}", non_local_ub);

            assert!(self.universal_regions.is_universal_region(non_local_ub));
            assert!(!self.universal_regions.is_local_free_region(non_local_ub));

            let requirement = ClosureOutlivesRequirement {
                subject,
                outlived_free_region: non_local_ub,
                blame_span: locations.span(mir),
                category: ConstraintCategory::Boring,
            };
            debug!("try_promote_type_test: pushing {:#?}", requirement);
            propagated_outlives_requirements.push(requirement);
        }
        true
    }

    /// When we promote a type test `T: 'r`, we have to convert the
    /// type `T` into something we can store in a query result (so
    /// something allocated for `'gcx`). This is problematic if `ty`
    /// contains regions. During the course of NLL region checking, we
    /// will have replaced all of those regions with fresh inference
    /// variables. To create a test subject, we want to replace those
    /// inference variables with some region from the closure
    /// signature -- this is not always possible, so this is a
    /// fallible process. Presuming we do find a suitable region, we
    /// will represent it with a `ReClosureBound`, which is a
    /// `RegionKind` variant that can be allocated in the gcx.
    fn try_promote_type_test_subject<'gcx>(
        &self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        ty: Ty<'tcx>,
    ) -> Option<ClosureOutlivesSubject<'gcx>> {
        let tcx = infcx.tcx;
        let gcx = tcx.global_tcx();

        debug!("try_promote_type_test_subject(ty = {:?})", ty);

        let ty = tcx.fold_regions(&ty, &mut false, |r, _depth| {
            let region_vid = self.to_region_vid(r);

            // The challenge if this. We have some region variable `r`
            // whose value is a set of CFG points and universal
            // regions. We want to find if that set is *equivalent* to
            // any of the named regions found in the closure.
            //
            // To do so, we compute the
            // `non_local_universal_upper_bound`. This will be a
            // non-local, universal region that is greater than `r`.
            // However, it might not be *contained* within `r`, so
            // then we further check whether this bound is contained
            // in `r`. If so, we can say that `r` is equivalent to the
            // bound.
            //
            // Let's work through a few examples. For these, imagine
            // that we have 3 non-local regions (I'll denote them as
            // `'static`, `'a`, and `'b`, though of course in the code
            // they would be represented with indices) where:
            //
            // - `'static: 'a`
            // - `'static: 'b`
            //
            // First, let's assume that `r` is some existential
            // variable with an inferred value `{'a, 'static}` (plus
            // some CFG nodes). In this case, the non-local upper
            // bound is `'static`, since that outlives `'a`. `'static`
            // is also a member of `r` and hence we consider `r`
            // equivalent to `'static` (and replace it with
            // `'static`).
            //
            // Now let's consider the inferred value `{'a, 'b}`. This
            // means `r` is effectively `'a | 'b`. I'm not sure if
            // this can come about, actually, but assuming it did, we
            // would get a non-local upper bound of `'static`. Since
            // `'static` is not contained in `r`, we would fail to
            // find an equivalent.
            let upper_bound = self.non_local_universal_upper_bound(region_vid);
            if self.region_contains(region_vid, upper_bound) {
                tcx.mk_region(ty::ReClosureBound(upper_bound))
            } else {
                // In the case of a failure, use a `ReVar`
                // result. This will cause the `lift` later on to
                // fail.
                r
            }
        });
        debug!("try_promote_type_test_subject: folded ty = {:?}", ty);

        // `lift` will only fail if we failed to promote some region.
        let ty = gcx.lift(&ty)?;

        Some(ClosureOutlivesSubject::Ty(ty))
    }

    /// Given some universal or existential region `r`, finds a
    /// non-local, universal region `r+` that outlives `r` at entry to (and
    /// exit from) the closure. In the worst case, this will be
    /// `'static`.
    ///
    /// This is used for two purposes. First, if we are propagated
    /// some requirement `T: r`, we can use this method to enlarge `r`
    /// to something we can encode for our creator (which only knows
    /// about non-local, universal regions). It is also used when
    /// encoding `T` as part of `try_promote_type_test_subject` (see
    /// that fn for details).
    ///
    /// This is based on the result `'y` of `universal_upper_bound`,
    /// except that it converts further takes the non-local upper
    /// bound of `'y`, so that the final result is non-local.
    fn non_local_universal_upper_bound(&self, r: RegionVid) -> RegionVid {
        debug!(
            "non_local_universal_upper_bound(r={:?}={})",
            r,
            self.region_value_str(r)
        );

        let lub = self.universal_upper_bound(r);

        // Grow further to get smallest universal region known to
        // creator.
        let non_local_lub = self.universal_region_relations.non_local_upper_bound(lub);

        debug!(
            "non_local_universal_upper_bound: non_local_lub={:?}",
            non_local_lub
        );

        non_local_lub
    }

    /// Returns a universally quantified region that outlives the
    /// value of `r` (`r` may be existentially or universally
    /// quantified).
    ///
    /// Since `r` is (potentially) an existential region, it has some
    /// value which may include (a) any number of points in the CFG
    /// and (b) any number of `end('x)` elements of universally
    /// quantified regions. To convert this into a single universal
    /// region we do as follows:
    ///
    /// - Ignore the CFG points in `'r`. All universally quantified regions
    ///   include the CFG anyhow.
    /// - For each `end('x)` element in `'r`, compute the mutual LUB, yielding
    ///   a result `'y`.
    fn universal_upper_bound(&self, r: RegionVid) -> RegionVid {
        debug!(
            "universal_upper_bound(r={:?}={})",
            r,
            self.region_value_str(r)
        );

        // Find the smallest universal region that contains all other
        // universal regions within `region`.
        let mut lub = self.universal_regions.fr_fn_body;
        let r_scc = self.constraint_sccs.scc(r);
        for ur in self.scc_values.universal_regions_outlived_by(r_scc) {
            lub = self.universal_region_relations.postdom_upper_bound(lub, ur);
        }

        debug!("universal_upper_bound: r={:?} lub={:?}", r, lub);

        lub
    }

    /// Test if `test` is true when applied to `lower_bound` at
    /// `point`, and returns true or false.
    fn eval_verify_bound(
        &self,
        tcx: TyCtxt<'_, '_, 'tcx>,
        mir: &Mir<'tcx>,
        generic_ty: Ty<'tcx>,
        lower_bound: RegionVid,
        verify_bound: &VerifyBound<'tcx>,
    ) -> bool {
        debug!(
            "eval_verify_bound(lower_bound={:?}, verify_bound={:?})",
            lower_bound, verify_bound
        );

        match verify_bound {
            VerifyBound::IfEq(test_ty, verify_bound1) => {
                self.eval_if_eq(tcx, mir, generic_ty, lower_bound, test_ty, verify_bound1)
            }

            VerifyBound::OutlivedBy(r) => {
                let r_vid = self.to_region_vid(r);
                self.eval_outlives(mir, r_vid, lower_bound)
            }

            VerifyBound::AnyBound(verify_bounds) => verify_bounds.iter().any(|verify_bound| {
                self.eval_verify_bound(tcx, mir, generic_ty, lower_bound, verify_bound)
            }),

            VerifyBound::AllBounds(verify_bounds) => verify_bounds.iter().all(|verify_bound| {
                self.eval_verify_bound(tcx, mir, generic_ty, lower_bound, verify_bound)
            }),
        }
    }

    fn eval_if_eq(
        &self,
        tcx: TyCtxt<'_, '_, 'tcx>,
        mir: &Mir<'tcx>,
        generic_ty: Ty<'tcx>,
        lower_bound: RegionVid,
        test_ty: Ty<'tcx>,
        verify_bound: &VerifyBound<'tcx>,
    ) -> bool {
        let generic_ty_normalized = self.normalize_to_scc_representatives(tcx, generic_ty);
        let test_ty_normalized = self.normalize_to_scc_representatives(tcx, test_ty);
        if generic_ty_normalized == test_ty_normalized {
            self.eval_verify_bound(tcx, mir, generic_ty, lower_bound, verify_bound)
        } else {
            false
        }
    }

    /// This is a conservative normalization procedure. It takes every
    /// free region in `value` and replaces it with the
    /// "representative" of its SCC (see `scc_representatives` field).
    /// We are guaranteed that if two values normalize to the same
    /// thing, then they are equal; this is a conservative check in
    /// that they could still be equal even if they normalize to
    /// different results. (For example, there might be two regions
    /// with the same value that are not in the same SCC).
    ///
    /// NB. This is not an ideal approach and I would like to revisit
    /// it. However, it works pretty well in practice. In particular,
    /// this is needed to deal with projection outlives bounds like
    ///
    ///     <T as Foo<'0>>::Item: '1
    ///
    /// In particular, this routine winds up being important when
    /// there are bounds like `where <T as Foo<'a>>::Item: 'b` in the
    /// environment.  In this case, if we can show that `'0 == 'a`,
    /// and that `'b: '1`, then we know that the clause is
    /// satisfied. In such cases, particularly due to limitations of
    /// the trait solver =), we usually wind up with a where-clause like
    /// `T: Foo<'a>` in scope, which thus forces `'0 == 'a` to be added as
    /// a constraint, and thus ensures that they are in the same SCC.
    ///
    /// So why can't we do a more correct routine? Well, we could
    /// *almost* use the `relate_tys` code, but the way it is
    /// currently setup it creates inference variables to deal with
    /// higher-ranked things and so forth, and right now the inference
    /// context is not permitted to make more inference variables. So
    /// we use this kind of hacky solution.
    fn normalize_to_scc_representatives<T>(&self, tcx: TyCtxt<'_, '_, 'tcx>, value: T) -> T
    where
        T: TypeFoldable<'tcx>,
    {
        tcx.fold_regions(&value, &mut false, |r, _db| {
            let vid = self.to_region_vid(r);
            let scc = self.constraint_sccs.scc(vid);
            let repr = self.scc_representatives[scc];
            tcx.mk_region(ty::ReVar(repr))
        })
    }

    // Evaluate whether `sup_region: sub_region @ point`.
    fn eval_outlives(
        &self,
        _mir: &Mir<'tcx>,
        sup_region: RegionVid,
        sub_region: RegionVid,
    ) -> bool {
        debug!("eval_outlives({:?}: {:?})", sup_region, sub_region);

        debug!(
            "eval_outlives: sup_region's value = {:?}",
            self.region_value_str(sup_region),
        );
        debug!(
            "eval_outlives: sub_region's value = {:?}",
            self.region_value_str(sub_region),
        );

        let sub_region_scc = self.constraint_sccs.scc(sub_region);
        let sup_region_scc = self.constraint_sccs.scc(sup_region);

        // Both the `sub_region` and `sup_region` consist of the union
        // of some number of universal regions (along with the union
        // of various points in the CFG; ignore those points for
        // now). Therefore, the sup-region outlives the sub-region if,
        // for each universal region R1 in the sub-region, there
        // exists some region R2 in the sup-region that outlives R1.
        let universal_outlives = self.scc_values
            .universal_regions_outlived_by(sub_region_scc)
            .all(|r1| {
                self.scc_values
                    .universal_regions_outlived_by(sup_region_scc)
                    .any(|r2| self.universal_region_relations.outlives(r2, r1))
            });

        if !universal_outlives {
            return false;
        }

        // Now we have to compare all the points in the sub region and make
        // sure they exist in the sup region.

        if self.universal_regions.is_universal_region(sup_region) {
            // Micro-opt: universal regions contain all points.
            return true;
        }

        self.scc_values
            .contains_points(sup_region_scc, sub_region_scc)
    }

    /// Once regions have been propagated, this method is used to see
    /// whether any of the constraints were too strong. In particular,
    /// we want to check for a case where a universally quantified
    /// region exceeded its bounds.  Consider:
    ///
    ///     fn foo<'a, 'b>(x: &'a u32) -> &'b u32 { x }
    ///
    /// In this case, returning `x` requires `&'a u32 <: &'b u32`
    /// and hence we establish (transitively) a constraint that
    /// `'a: 'b`. The `propagate_constraints` code above will
    /// therefore add `end('a)` into the region for `'b` -- but we
    /// have no evidence that `'b` outlives `'a`, so we want to report
    /// an error.
    ///
    /// If `propagated_outlives_requirements` is `Some`, then we will
    /// push unsatisfied obligations into there. Otherwise, we'll
    /// report them as errors.
    fn check_universal_regions<'gcx>(
        &self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        mir_def_id: DefId,
        mut propagated_outlives_requirements: Option<&mut Vec<ClosureOutlivesRequirement<'gcx>>>,
        errors_buffer: &mut Vec<Diagnostic>,
    ) {
        for (fr, fr_definition) in self.definitions.iter_enumerated() {
            match fr_definition.origin {
                NLLRegionVariableOrigin::FreeRegion => {
                    // Go through each of the universal regions `fr` and check that
                    // they did not grow too large, accumulating any requirements
                    // for our caller into the `outlives_requirements` vector.
                    self.check_universal_region(
                        infcx,
                        mir,
                        mir_def_id,
                        fr,
                        &mut propagated_outlives_requirements,
                        errors_buffer,
                    );
                }

                NLLRegionVariableOrigin::Placeholder(placeholder) => {
                    self.check_bound_universal_region(infcx, mir, mir_def_id, fr, placeholder);
                }

                NLLRegionVariableOrigin::Existential => {
                    // nothing to check here
                }
            }
        }
    }

    /// Check the final value for the free region `fr` to see if it
    /// grew too large. In particular, examine what `end(X)` points
    /// wound up in `fr`'s final value; for each `end(X)` where `X !=
    /// fr`, we want to check that `fr: X`. If not, that's either an
    /// error, or something we have to propagate to our creator.
    ///
    /// Things that are to be propagated are accumulated into the
    /// `outlives_requirements` vector.
    fn check_universal_region<'gcx>(
        &self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        mir_def_id: DefId,
        longer_fr: RegionVid,
        propagated_outlives_requirements: &mut Option<&mut Vec<ClosureOutlivesRequirement<'gcx>>>,
        errors_buffer: &mut Vec<Diagnostic>,
    ) {
        debug!("check_universal_region(fr={:?})", longer_fr);

        let longer_fr_scc = self.constraint_sccs.scc(longer_fr);

        // Because this free region must be in the ROOT universe, we
        // know it cannot contain any bound universes.
        assert!(self.scc_universes[longer_fr_scc] == ty::UniverseIndex::ROOT);
        debug_assert!(
            self.scc_values
                .placeholders_contained_in(longer_fr_scc)
                .next()
                .is_none()
        );

        // Find every region `o` such that `fr: o`
        // (because `fr` includes `end(o)`).
        for shorter_fr in self.scc_values.universal_regions_outlived_by(longer_fr_scc) {
            // If it is known that `fr: o`, carry on.
            if self.universal_region_relations
                .outlives(longer_fr, shorter_fr)
            {
                continue;
            }

            debug!(
                "check_universal_region: fr={:?} does not outlive shorter_fr={:?}",
                longer_fr, shorter_fr,
            );

            let blame_span_category = self.find_outlives_blame_span(mir, longer_fr, shorter_fr);

            if let Some(propagated_outlives_requirements) = propagated_outlives_requirements {
                // Shrink `fr` until we find a non-local region (if we do).
                // We'll call that `fr-` -- it's ever so slightly smaller than `fr`.
                if let Some(fr_minus) = self.universal_region_relations
                    .non_local_lower_bound(longer_fr)
                {
                    debug!("check_universal_region: fr_minus={:?}", fr_minus);

                    // Grow `shorter_fr` until we find a non-local
                    // region. (We always will.)  We'll call that
                    // `shorter_fr+` -- it's ever so slightly larger than
                    // `fr`.
                    let shorter_fr_plus = self.universal_region_relations
                        .non_local_upper_bound(shorter_fr);
                    debug!(
                        "check_universal_region: shorter_fr_plus={:?}",
                        shorter_fr_plus
                    );

                    // Push the constraint `fr-: shorter_fr+`
                    propagated_outlives_requirements.push(ClosureOutlivesRequirement {
                        subject: ClosureOutlivesSubject::Region(fr_minus),
                        outlived_free_region: shorter_fr_plus,
                        blame_span: blame_span_category.1,
                        category: blame_span_category.0,
                    });
                    continue;
                }
            }

            // If we are not in a context where we can propagate
            // errors, or we could not shrink `fr` to something
            // smaller, then just report an error.
            //
            // Note: in this case, we use the unapproximated regions
            // to report the error. This gives better error messages
            // in some cases.
            self.report_error(mir, infcx, mir_def_id, longer_fr, shorter_fr, errors_buffer);
            return; // continuing to iterate just reports more errors than necessary
        }
    }

    fn check_bound_universal_region<'gcx>(
        &self,
        infcx: &InferCtxt<'_, 'gcx, 'tcx>,
        mir: &Mir<'tcx>,
        _mir_def_id: DefId,
        longer_fr: RegionVid,
        placeholder: ty::PlaceholderRegion,
    ) {
        debug!(
            "check_bound_universal_region(fr={:?}, placeholder={:?})",
            longer_fr, placeholder,
        );

        let longer_fr_scc = self.constraint_sccs.scc(longer_fr);
        debug!(
            "check_bound_universal_region: longer_fr_scc={:?}",
            longer_fr_scc,
        );

        // If we have some bound universal region `'a`, then the only
        // elements it can contain is itself -- we don't know anything
        // else about it!
        let error_element = match {
            self.scc_values
                .elements_contained_in(longer_fr_scc)
                .find(|element| match element {
                    RegionElement::Location(_) => true,
                    RegionElement::RootUniversalRegion(_) => true,
                    RegionElement::PlaceholderRegion(placeholder1) => placeholder != *placeholder1,
                })
        } {
            Some(v) => v,
            None => return,
        };
        debug!("check_bound_universal_region: error_element = {:?}", error_element);

        // Find the region that introduced this `error_element`.
        let error_region = match error_element {
            RegionElement::Location(l) => self.find_sub_region_live_at(longer_fr, l),
            RegionElement::RootUniversalRegion(r) => r,
            RegionElement::PlaceholderRegion(error_placeholder) => self.definitions
                .iter_enumerated()
                .filter_map(|(r, definition)| match definition.origin {
                    NLLRegionVariableOrigin::Placeholder(p) if p == error_placeholder => Some(r),
                    _ => None,
                })
                .next()
                .unwrap(),
        };

        // Find the code to blame for the fact that `longer_fr` outlives `error_fr`.
        let (_, span) = self.find_outlives_blame_span(mir, longer_fr, error_region);

        // Obviously, this error message is far from satisfactory.
        // At present, though, it only appears in unit tests --
        // the AST-based checker uses a more conservative check,
        // so to even see this error, one must pass in a special
        // flag.
        let mut diag = infcx
            .tcx
            .sess
            .struct_span_err(span, "higher-ranked subtype error");
        diag.emit();
    }
}

impl<'tcx> RegionDefinition<'tcx> {
    fn new(universe: ty::UniverseIndex, rv_origin: RegionVariableOrigin) -> Self {
        // Create a new region definition. Note that, for free
        // regions, the `external_name` field gets updated later in
        // `init_universal_regions`.

        let origin = match rv_origin {
            RegionVariableOrigin::NLL(origin) => origin,
            _ => NLLRegionVariableOrigin::Existential,
        };

        Self {
            origin,
            universe,
            external_name: None,
        }
    }
}

pub trait ClosureRegionRequirementsExt<'gcx, 'tcx> {
    fn apply_requirements(
        &self,
        tcx: TyCtxt<'_, 'gcx, 'tcx>,
        location: Location,
        closure_def_id: DefId,
        closure_substs: &'tcx ty::subst::Substs<'tcx>,
    ) -> Vec<QueryRegionConstraint<'tcx>>;

    fn subst_closure_mapping<T>(
        &self,
        tcx: TyCtxt<'_, 'gcx, 'tcx>,
        closure_mapping: &IndexVec<RegionVid, ty::Region<'tcx>>,
        value: &T,
    ) -> T
    where
        T: TypeFoldable<'tcx>;
}

impl<'gcx, 'tcx> ClosureRegionRequirementsExt<'gcx, 'tcx> for ClosureRegionRequirements<'gcx> {
    /// Given an instance T of the closure type, this method
    /// instantiates the "extra" requirements that we computed for the
    /// closure into the inference context. This has the effect of
    /// adding new outlives obligations to existing variables.
    ///
    /// As described on `ClosureRegionRequirements`, the extra
    /// requirements are expressed in terms of regionvids that index
    /// into the free regions that appear on the closure type. So, to
    /// do this, we first copy those regions out from the type T into
    /// a vector. Then we can just index into that vector to extract
    /// out the corresponding region from T and apply the
    /// requirements.
    fn apply_requirements(
        &self,
        tcx: TyCtxt<'_, 'gcx, 'tcx>,
        location: Location,
        closure_def_id: DefId,
        closure_substs: &'tcx ty::subst::Substs<'tcx>,
    ) -> Vec<QueryRegionConstraint<'tcx>> {
        debug!(
            "apply_requirements(location={:?}, closure_def_id={:?}, closure_substs={:?})",
            location, closure_def_id, closure_substs
        );

        // Extract the values of the free regions in `closure_substs`
        // into a vector.  These are the regions that we will be
        // relating to one another.
        let closure_mapping = &UniversalRegions::closure_mapping(
            tcx,
            closure_substs,
            self.num_external_vids,
            tcx.closure_base_def_id(closure_def_id),
        );
        debug!("apply_requirements: closure_mapping={:?}", closure_mapping);

        // Create the predicates.
        self.outlives_requirements
            .iter()
            .map(|outlives_requirement| {
                let outlived_region = closure_mapping[outlives_requirement.outlived_free_region];

                match outlives_requirement.subject {
                    ClosureOutlivesSubject::Region(region) => {
                        let region = closure_mapping[region];
                        debug!(
                            "apply_requirements: region={:?} \
                             outlived_region={:?} \
                             outlives_requirement={:?}",
                            region, outlived_region, outlives_requirement,
                        );
                        ty::Binder::dummy(ty::OutlivesPredicate(region.into(), outlived_region))
                    }

                    ClosureOutlivesSubject::Ty(ty) => {
                        let ty = self.subst_closure_mapping(tcx, closure_mapping, &ty);
                        debug!(
                            "apply_requirements: ty={:?} \
                             outlived_region={:?} \
                             outlives_requirement={:?}",
                            ty, outlived_region, outlives_requirement,
                        );
                        ty::Binder::dummy(ty::OutlivesPredicate(ty.into(), outlived_region))
                    }
                }
            })
            .collect()
    }

    fn subst_closure_mapping<T>(
        &self,
        tcx: TyCtxt<'_, 'gcx, 'tcx>,
        closure_mapping: &IndexVec<RegionVid, ty::Region<'tcx>>,
        value: &T,
    ) -> T
    where
        T: TypeFoldable<'tcx>,
    {
        tcx.fold_regions(value, &mut false, |r, _depth| {
            if let ty::ReClosureBound(vid) = r {
                closure_mapping[*vid]
            } else {
                bug!(
                    "subst_closure_mapping: encountered non-closure bound free region {:?}",
                    r
                )
            }
        })
    }
}
