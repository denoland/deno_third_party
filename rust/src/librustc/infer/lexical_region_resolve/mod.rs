//! The code to do lexical region resolution.

use infer::region_constraints::Constraint;
use infer::region_constraints::GenericKind;
use infer::region_constraints::RegionConstraintData;
use infer::region_constraints::VarInfos;
use infer::region_constraints::VerifyBound;
use infer::RegionVariableOrigin;
use infer::SubregionOrigin;
use middle::free_region::RegionRelations;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::graph::implementation::{
    Direction, Graph, NodeIndex, INCOMING, OUTGOING,
};
use rustc_data_structures::indexed_vec::{Idx, IndexVec};
use std::fmt;
use std::u32;
use ty::fold::TypeFoldable;
use ty::{self, Ty, TyCtxt};
use ty::{ReEarlyBound, ReEmpty, ReErased, ReFree, ReStatic};
use ty::{ReLateBound, ReScope, RePlaceholder, ReVar};
use ty::{Region, RegionVid};

mod graphviz;

/// This function performs lexical region resolution given a complete
/// set of constraints and variable origins. It performs a fixed-point
/// iteration to find region values which satisfy all constraints,
/// assuming such values can be found. It returns the final values of
/// all the variables as well as a set of errors that must be reported.
pub fn resolve<'tcx>(
    region_rels: &RegionRelations<'_, '_, 'tcx>,
    var_infos: VarInfos,
    data: RegionConstraintData<'tcx>,
) -> (
    LexicalRegionResolutions<'tcx>,
    Vec<RegionResolutionError<'tcx>>,
) {
    debug!("RegionConstraintData: resolve_regions()");
    let mut errors = vec![];
    let mut resolver = LexicalResolver {
        region_rels,
        var_infos,
        data,
    };
    let values = resolver.infer_variable_values(&mut errors);
    (values, errors)
}

/// Contains the result of lexical region resolution. Offers methods
/// to lookup up the final value of a region variable.
pub struct LexicalRegionResolutions<'tcx> {
    values: IndexVec<RegionVid, VarValue<'tcx>>,
    error_region: ty::Region<'tcx>,
}

#[derive(Copy, Clone, Debug)]
enum VarValue<'tcx> {
    Value(Region<'tcx>),
    ErrorValue,
}

#[derive(Clone, Debug)]
pub enum RegionResolutionError<'tcx> {
    /// `ConcreteFailure(o, a, b)`:
    ///
    /// `o` requires that `a <= b`, but this does not hold
    ConcreteFailure(SubregionOrigin<'tcx>, Region<'tcx>, Region<'tcx>),

    /// `GenericBoundFailure(p, s, a)
    ///
    /// The parameter/associated-type `p` must be known to outlive the lifetime
    /// `a` (but none of the known bounds are sufficient).
    GenericBoundFailure(SubregionOrigin<'tcx>, GenericKind<'tcx>, Region<'tcx>),

    /// `SubSupConflict(v, v_origin, sub_origin, sub_r, sup_origin, sup_r)`:
    ///
    /// Could not infer a value for `v` (which has origin `v_origin`)
    /// because `sub_r <= v` (due to `sub_origin`) but `v <= sup_r` (due to `sup_origin`) and
    /// `sub_r <= sup_r` does not hold.
    SubSupConflict(
        RegionVid,
        RegionVariableOrigin,
        SubregionOrigin<'tcx>,
        Region<'tcx>,
        SubregionOrigin<'tcx>,
        Region<'tcx>,
    ),
}

struct RegionAndOrigin<'tcx> {
    region: Region<'tcx>,
    origin: SubregionOrigin<'tcx>,
}

type RegionGraph<'tcx> = Graph<(), Constraint<'tcx>>;

struct LexicalResolver<'cx, 'gcx: 'tcx, 'tcx: 'cx> {
    region_rels: &'cx RegionRelations<'cx, 'gcx, 'tcx>,
    var_infos: VarInfos,
    data: RegionConstraintData<'tcx>,
}

impl<'cx, 'gcx, 'tcx> LexicalResolver<'cx, 'gcx, 'tcx> {
    fn tcx(&self) -> TyCtxt<'cx, 'gcx, 'tcx> {
        self.region_rels.tcx
    }

    fn infer_variable_values(
        &mut self,
        errors: &mut Vec<RegionResolutionError<'tcx>>,
    ) -> LexicalRegionResolutions<'tcx> {
        let mut var_data = self.construct_var_data(self.tcx());

        // Dorky hack to cause `dump_constraints` to only get called
        // if debug mode is enabled:
        debug!(
            "----() End constraint listing (context={:?}) {:?}---",
            self.region_rels.context,
            self.dump_constraints(self.region_rels)
        );
        graphviz::maybe_print_constraints_for(&self.data, self.region_rels);

        let graph = self.construct_graph();
        self.expand_givens(&graph);
        self.expansion(&mut var_data);
        self.collect_errors(&mut var_data, errors);
        self.collect_var_errors(&var_data, &graph, errors);
        var_data
    }

    fn num_vars(&self) -> usize {
        self.var_infos.len()
    }

    /// Initially, the value for all variables is set to `'empty`, the
    /// empty region. The `expansion` phase will grow this larger.
    fn construct_var_data(&self, tcx: TyCtxt<'_, '_, 'tcx>) -> LexicalRegionResolutions<'tcx> {
        LexicalRegionResolutions {
            error_region: tcx.types.re_static,
            values: IndexVec::from_elem_n(VarValue::Value(tcx.types.re_empty), self.num_vars())
        }
    }

    fn dump_constraints(&self, free_regions: &RegionRelations<'_, '_, 'tcx>) {
        debug!(
            "----() Start constraint listing (context={:?}) ()----",
            free_regions.context
        );
        for (idx, (constraint, _)) in self.data.constraints.iter().enumerate() {
            debug!("Constraint {} => {:?}", idx, constraint);
        }
    }

    fn expand_givens(&mut self, graph: &RegionGraph<'_>) {
        // Givens are a kind of horrible hack to account for
        // constraints like 'c <= '0 that are known to hold due to
        // closure signatures (see the comment above on the `givens`
        // field). They should go away. But until they do, the role
        // of this fn is to account for the transitive nature:
        //
        //     Given 'c <= '0
        //     and   '0 <= '1
        //     then  'c <= '1

        let seeds: Vec<_> = self.data.givens.iter().cloned().collect();
        for (r, vid) in seeds {
            // While all things transitively reachable in the graph
            // from the variable (`'0` in the example above).
            let seed_index = NodeIndex(vid.index() as usize);
            for succ_index in graph.depth_traverse(seed_index, OUTGOING) {
                let succ_index = succ_index.0;

                // The first N nodes correspond to the region
                // variables. Other nodes correspond to constant
                // regions.
                if succ_index < self.num_vars() {
                    let succ_vid = RegionVid::new(succ_index);

                    // Add `'c <= '1`.
                    self.data.givens.insert((r, succ_vid));
                }
            }
        }
    }

    fn expansion(&self, var_values: &mut LexicalRegionResolutions<'tcx>) {
        self.iterate_until_fixed_point("Expansion", |constraint, origin| {
            debug!("expansion: constraint={:?} origin={:?}", constraint, origin);
            match *constraint {
                Constraint::RegSubVar(a_region, b_vid) => {
                    let b_data = var_values.value_mut(b_vid);
                    self.expand_node(a_region, b_vid, b_data)
                }
                Constraint::VarSubVar(a_vid, b_vid) => match *var_values.value(a_vid) {
                    VarValue::ErrorValue => false,
                    VarValue::Value(a_region) => {
                        let b_node = var_values.value_mut(b_vid);
                        self.expand_node(a_region, b_vid, b_node)
                    }
                },
                Constraint::RegSubReg(..) | Constraint::VarSubReg(..) => {
                    // These constraints are checked after expansion
                    // is done, in `collect_errors`.
                    false
                }
            }
        })
    }

    fn expand_node(
        &self,
        a_region: Region<'tcx>,
        b_vid: RegionVid,
        b_data: &mut VarValue<'tcx>,
    ) -> bool {
        debug!("expand_node({:?}, {:?} == {:?})", a_region, b_vid, b_data);

        match *a_region {
            // Check if this relationship is implied by a given.
            ty::ReEarlyBound(_) | ty::ReFree(_) => if self.data.givens.contains(&(a_region, b_vid))
            {
                debug!("given");
                return false;
            },

            _ => {}
        }


        match *b_data {
            VarValue::Value(cur_region) => {
                let mut lub = self.lub_concrete_regions(a_region, cur_region);
                if lub == cur_region {
                    return false;
                }

                // Watch out for `'b: !1` relationships, where the
                // universe of `'b` can't name the placeholder `!1`. In
                // that case, we have to grow `'b` to be `'static` for the
                // relationship to hold. This is obviously a kind of sub-optimal
                // choice -- in the future, when we incorporate a knowledge
                // of the parameter environment, we might be able to find a
                // tighter bound than `'static`.
                //
                // (This might e.g. arise from being asked to prove `for<'a> { 'b: 'a }`.)
                let b_universe = self.var_infos[b_vid].universe;
                if let ty::RePlaceholder(p) = lub {
                    if b_universe.cannot_name(p.universe) {
                        lub = self.tcx().types.re_static;
                    }
                }

                debug!(
                    "Expanding value of {:?} from {:?} to {:?}",
                    b_vid, cur_region, lub
                );

                *b_data = VarValue::Value(lub);
                return true;
            }

            VarValue::ErrorValue => {
                return false;
            }
        }
    }

    fn lub_concrete_regions(&self, a: Region<'tcx>, b: Region<'tcx>) -> Region<'tcx> {
        let tcx = self.tcx();
        match (a, b) {
            (&ty::ReClosureBound(..), _)
            | (_, &ty::ReClosureBound(..))
            | (&ReLateBound(..), _)
            | (_, &ReLateBound(..))
            | (&ReErased, _)
            | (_, &ReErased) => {
                bug!("cannot relate region: LUB({:?}, {:?})", a, b);
            }

            (r @ &ReStatic, _) | (_, r @ &ReStatic) => {
                r // nothing lives longer than static
            }

            (&ReEmpty, r) | (r, &ReEmpty) => {
                r // everything lives longer than empty
            }

            (&ReVar(v_id), _) | (_, &ReVar(v_id)) => {
                span_bug!(
                    self.var_infos[v_id].origin.span(),
                    "lub_concrete_regions invoked with non-concrete \
                     regions: {:?}, {:?}",
                    a,
                    b
                );
            }

            (&ReEarlyBound(_), &ReScope(s_id))
            | (&ReScope(s_id), &ReEarlyBound(_))
            | (&ReFree(_), &ReScope(s_id))
            | (&ReScope(s_id), &ReFree(_)) => {
                // A "free" region can be interpreted as "some region
                // at least as big as fr.scope".  So, we can
                // reasonably compare free regions and scopes:
                let fr_scope = match (a, b) {
                    (&ReEarlyBound(ref br), _) | (_, &ReEarlyBound(ref br)) => self.region_rels
                        .region_scope_tree
                        .early_free_scope(self.tcx(), br),
                    (&ReFree(ref fr), _) | (_, &ReFree(ref fr)) => self.region_rels
                        .region_scope_tree
                        .free_scope(self.tcx(), fr),
                    _ => bug!(),
                };
                let r_id = self.region_rels
                    .region_scope_tree
                    .nearest_common_ancestor(fr_scope, s_id);
                if r_id == fr_scope {
                    // if the free region's scope `fr.scope` is bigger than
                    // the scope region `s_id`, then the LUB is the free
                    // region itself:
                    match (a, b) {
                        (_, &ReScope(_)) => return a,
                        (&ReScope(_), _) => return b,
                        _ => bug!(),
                    }
                }

                // otherwise, we don't know what the free region is,
                // so we must conservatively say the LUB is static:
                tcx.types.re_static
            }

            (&ReScope(a_id), &ReScope(b_id)) => {
                // The region corresponding to an outer block is a
                // subtype of the region corresponding to an inner
                // block.
                let lub = self.region_rels
                    .region_scope_tree
                    .nearest_common_ancestor(a_id, b_id);
                tcx.mk_region(ReScope(lub))
            }

            (&ReEarlyBound(_), &ReEarlyBound(_))
            | (&ReFree(_), &ReEarlyBound(_))
            | (&ReEarlyBound(_), &ReFree(_))
            | (&ReFree(_), &ReFree(_)) => self.region_rels.lub_free_regions(a, b),

            // For these types, we cannot define any additional
            // relationship:
            (&RePlaceholder(..), _) | (_, &RePlaceholder(..)) => if a == b {
                a
            } else {
                tcx.types.re_static
            },
        }
    }

    /// After expansion is complete, go and check upper bounds (i.e.,
    /// cases where the region cannot grow larger than a fixed point)
    /// and check that they are satisfied.
    fn collect_errors(
        &self,
        var_data: &mut LexicalRegionResolutions<'tcx>,
        errors: &mut Vec<RegionResolutionError<'tcx>>,
    ) {
        for (constraint, origin) in &self.data.constraints {
            debug!(
                "collect_errors: constraint={:?} origin={:?}",
                constraint, origin
            );
            match *constraint {
                Constraint::RegSubVar(..) | Constraint::VarSubVar(..) => {
                    // Expansion will ensure that these constraints hold. Ignore.
                }

                Constraint::RegSubReg(sub, sup) => {
                    if self.region_rels.is_subregion_of(sub, sup) {
                        continue;
                    }

                    debug!(
                        "collect_errors: region error at {:?}: \
                         cannot verify that {:?} <= {:?}",
                        origin, sub, sup
                    );

                    errors.push(RegionResolutionError::ConcreteFailure(
                        (*origin).clone(),
                        sub,
                        sup,
                    ));
                }

                Constraint::VarSubReg(a_vid, b_region) => {
                    let a_data = var_data.value_mut(a_vid);
                    debug!("contraction: {:?} == {:?}, {:?}", a_vid, a_data, b_region);

                    let a_region = match *a_data {
                        VarValue::ErrorValue => continue,
                        VarValue::Value(a_region) => a_region,
                    };

                    // Do not report these errors immediately:
                    // instead, set the variable value to error and
                    // collect them later.
                    if !self.region_rels.is_subregion_of(a_region, b_region) {
                        debug!(
                            "collect_errors: region error at {:?}: \
                             cannot verify that {:?}={:?} <= {:?}",
                            origin, a_vid, a_region, b_region
                        );
                        *a_data = VarValue::ErrorValue;
                    }
                }
            }
        }

        for verify in &self.data.verifys {
            debug!("collect_errors: verify={:?}", verify);
            let sub = var_data.normalize(self.tcx(), verify.region);

            // This was an inference variable which didn't get
            // constrained, therefore it can be assume to hold.
            if let ty::ReEmpty = *sub {
                continue;
            }

            let verify_kind_ty = verify.kind.to_ty(self.tcx());
            if self.bound_is_met(&verify.bound, var_data, verify_kind_ty, sub) {
                continue;
            }

            debug!(
                "collect_errors: region error at {:?}: \
                 cannot verify that {:?} <= {:?}",
                verify.origin, verify.region, verify.bound
            );

            errors.push(RegionResolutionError::GenericBoundFailure(
                verify.origin.clone(),
                verify.kind.clone(),
                sub,
            ));
        }
    }

    /// Go over the variables that were declared to be error variables
    /// and create a `RegionResolutionError` for each of them.
    fn collect_var_errors(
        &self,
        var_data: &LexicalRegionResolutions<'tcx>,
        graph: &RegionGraph<'tcx>,
        errors: &mut Vec<RegionResolutionError<'tcx>>,
    ) {
        debug!("collect_var_errors");

        // This is the best way that I have found to suppress
        // duplicate and related errors. Basically we keep a set of
        // flags for every node. Whenever an error occurs, we will
        // walk some portion of the graph looking to find pairs of
        // conflicting regions to report to the user. As we walk, we
        // trip the flags from false to true, and if we find that
        // we've already reported an error involving any particular
        // node we just stop and don't report the current error.  The
        // idea is to report errors that derive from independent
        // regions of the graph, but not those that derive from
        // overlapping locations.
        let mut dup_vec = vec![u32::MAX; self.num_vars()];

        for (node_vid, value) in var_data.values.iter_enumerated() {
            match *value {
                VarValue::Value(_) => { /* Inference successful */ }
                VarValue::ErrorValue => {
                    /* Inference impossible, this value contains
                       inconsistent constraints.

                       I think that in this case we should report an
                       error now---unlike the case above, we can't
                       wait to see whether the user needs the result
                       of this variable.  The reason is that the mere
                       existence of this variable implies that the
                       region graph is inconsistent, whether or not it
                       is used.

                       For example, we may have created a region
                       variable that is the GLB of two other regions
                       which do not have a GLB.  Even if that variable
                       is not used, it implies that those two regions
                       *should* have a GLB.

                       At least I think this is true. It may be that
                       the mere existence of a conflict in a region variable
                       that is not used is not a problem, so if this rule
                       starts to create problems we'll have to revisit
                       this portion of the code and think hard about it. =) */
                    self.collect_error_for_expanding_node(graph, &mut dup_vec, node_vid, errors);
                }
            }
        }
    }

    fn construct_graph(&self) -> RegionGraph<'tcx> {
        let num_vars = self.num_vars();

        let mut graph = Graph::new();

        for _ in 0..num_vars {
            graph.add_node(());
        }

        // Issue #30438: two distinct dummy nodes, one for incoming
        // edges (dummy_source) and another for outgoing edges
        // (dummy_sink). In `dummy -> a -> b -> dummy`, using one
        // dummy node leads one to think (erroneously) there exists a
        // path from `b` to `a`. Two dummy nodes sidesteps the issue.
        let dummy_source = graph.add_node(());
        let dummy_sink = graph.add_node(());

        for (constraint, _) in &self.data.constraints {
            match *constraint {
                Constraint::VarSubVar(a_id, b_id) => {
                    graph.add_edge(
                        NodeIndex(a_id.index() as usize),
                        NodeIndex(b_id.index() as usize),
                        *constraint,
                    );
                }
                Constraint::RegSubVar(_, b_id) => {
                    graph.add_edge(dummy_source, NodeIndex(b_id.index() as usize), *constraint);
                }
                Constraint::VarSubReg(a_id, _) => {
                    graph.add_edge(NodeIndex(a_id.index() as usize), dummy_sink, *constraint);
                }
                Constraint::RegSubReg(..) => {
                    // this would be an edge from `dummy_source` to
                    // `dummy_sink`; just ignore it.
                }
            }
        }

        return graph;
    }

    fn collect_error_for_expanding_node(
        &self,
        graph: &RegionGraph<'tcx>,
        dup_vec: &mut [u32],
        node_idx: RegionVid,
        errors: &mut Vec<RegionResolutionError<'tcx>>,
    ) {
        // Errors in expanding nodes result from a lower-bound that is
        // not contained by an upper-bound.
        let (mut lower_bounds, lower_dup) =
            self.collect_concrete_regions(graph, node_idx, INCOMING, dup_vec);
        let (mut upper_bounds, upper_dup) =
            self.collect_concrete_regions(graph, node_idx, OUTGOING, dup_vec);

        if lower_dup || upper_dup {
            return;
        }

        // We place free regions first because we are special casing
        // SubSupConflict(ReFree, ReFree) when reporting error, and so
        // the user will more likely get a specific suggestion.
        fn region_order_key(x: &RegionAndOrigin<'_>) -> u8 {
            match *x.region {
                ReEarlyBound(_) => 0,
                ReFree(_) => 1,
                _ => 2,
            }
        }
        lower_bounds.sort_by_key(region_order_key);
        upper_bounds.sort_by_key(region_order_key);

        let node_universe = self.var_infos[node_idx].universe;

        for lower_bound in &lower_bounds {
            let effective_lower_bound = if let ty::RePlaceholder(p) = lower_bound.region {
                if node_universe.cannot_name(p.universe) {
                    self.tcx().types.re_static
                } else {
                    lower_bound.region
                }
            } else {
                lower_bound.region
            };

            for upper_bound in &upper_bounds {
                if !self.region_rels
                    .is_subregion_of(effective_lower_bound, upper_bound.region)
                {
                    let origin = self.var_infos[node_idx].origin.clone();
                    debug!(
                        "region inference error at {:?} for {:?}: SubSupConflict sub: {:?} \
                         sup: {:?}",
                        origin, node_idx, lower_bound.region, upper_bound.region
                    );
                    errors.push(RegionResolutionError::SubSupConflict(
                        node_idx,
                        origin,
                        lower_bound.origin.clone(),
                        lower_bound.region,
                        upper_bound.origin.clone(),
                        upper_bound.region,
                    ));
                    return;
                }
            }
        }

        span_bug!(
            self.var_infos[node_idx].origin.span(),
            "collect_error_for_expanding_node() could not find \
             error for var {:?} in universe {:?}, lower_bounds={:#?}, \
             upper_bounds={:#?}",
            node_idx,
            node_universe,
            lower_bounds,
            upper_bounds
        );
    }

    fn collect_concrete_regions(
        &self,
        graph: &RegionGraph<'tcx>,
        orig_node_idx: RegionVid,
        dir: Direction,
        dup_vec: &mut [u32],
    ) -> (Vec<RegionAndOrigin<'tcx>>, bool) {
        struct WalkState<'tcx> {
            set: FxHashSet<RegionVid>,
            stack: Vec<RegionVid>,
            result: Vec<RegionAndOrigin<'tcx>>,
            dup_found: bool,
        }
        let mut state = WalkState {
            set: Default::default(),
            stack: vec![orig_node_idx],
            result: Vec::new(),
            dup_found: false,
        };
        state.set.insert(orig_node_idx);

        // to start off the process, walk the source node in the
        // direction specified
        process_edges(&self.data, &mut state, graph, orig_node_idx, dir);

        while !state.stack.is_empty() {
            let node_idx = state.stack.pop().unwrap();

            // check whether we've visited this node on some previous walk
            if dup_vec[node_idx.index() as usize] == u32::MAX {
                dup_vec[node_idx.index() as usize] = orig_node_idx.index() as u32;
            } else if dup_vec[node_idx.index() as usize] != orig_node_idx.index() as u32 {
                state.dup_found = true;
            }

            debug!(
                "collect_concrete_regions(orig_node_idx={:?}, node_idx={:?})",
                orig_node_idx, node_idx
            );

            process_edges(&self.data, &mut state, graph, node_idx, dir);
        }

        let WalkState {
            result, dup_found, ..
        } = state;
        return (result, dup_found);

        fn process_edges<'tcx>(
            this: &RegionConstraintData<'tcx>,
            state: &mut WalkState<'tcx>,
            graph: &RegionGraph<'tcx>,
            source_vid: RegionVid,
            dir: Direction,
        ) {
            debug!("process_edges(source_vid={:?}, dir={:?})", source_vid, dir);

            let source_node_index = NodeIndex(source_vid.index() as usize);
            for (_, edge) in graph.adjacent_edges(source_node_index, dir) {
                match edge.data {
                    Constraint::VarSubVar(from_vid, to_vid) => {
                        let opp_vid = if from_vid == source_vid {
                            to_vid
                        } else {
                            from_vid
                        };
                        if state.set.insert(opp_vid) {
                            state.stack.push(opp_vid);
                        }
                    }

                    Constraint::RegSubVar(region, _) | Constraint::VarSubReg(_, region) => {
                        state.result.push(RegionAndOrigin {
                            region,
                            origin: this.constraints.get(&edge.data).unwrap().clone(),
                        });
                    }

                    Constraint::RegSubReg(..) => panic!(
                        "cannot reach reg-sub-reg edge in region inference \
                         post-processing"
                    ),
                }
            }
        }
    }

    fn iterate_until_fixed_point<F>(&self, tag: &str, mut body: F)
    where
        F: FnMut(&Constraint<'tcx>, &SubregionOrigin<'tcx>) -> bool,
    {
        let mut iteration = 0;
        let mut changed = true;
        while changed {
            changed = false;
            iteration += 1;
            debug!("---- {} Iteration {}{}", "#", tag, iteration);
            for (constraint, origin) in &self.data.constraints {
                let edge_changed = body(constraint, origin);
                if edge_changed {
                    debug!("Updated due to constraint {:?}", constraint);
                    changed = true;
                }
            }
        }
        debug!("---- {} Complete after {} iteration(s)", tag, iteration);
    }

    fn bound_is_met(
        &self,
        bound: &VerifyBound<'tcx>,
        var_values: &LexicalRegionResolutions<'tcx>,
        generic_ty: Ty<'tcx>,
        min: ty::Region<'tcx>,
    ) -> bool {
        match bound {
            VerifyBound::IfEq(k, b) => {
                (var_values.normalize(self.region_rels.tcx, *k) == generic_ty)
                    && self.bound_is_met(b, var_values, generic_ty, min)
            }

            VerifyBound::OutlivedBy(r) =>
                self.region_rels.is_subregion_of(
                    min,
                    var_values.normalize(self.tcx(), r),
                ),

            VerifyBound::AnyBound(bs) => bs.iter()
                .any(|b| self.bound_is_met(b, var_values, generic_ty, min)),

            VerifyBound::AllBounds(bs) => bs.iter()
                .all(|b| self.bound_is_met(b, var_values, generic_ty, min)),
        }
    }
}

impl<'tcx> fmt::Debug for RegionAndOrigin<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RegionAndOrigin({:?},{:?})", self.region, self.origin)
    }
}

impl<'tcx> LexicalRegionResolutions<'tcx> {
    fn normalize<T>(&self, tcx: TyCtxt<'_, '_, 'tcx>, value: T) -> T
    where
        T: TypeFoldable<'tcx>,
    {
        tcx.fold_regions(&value, &mut false, |r, _db| match r {
            ty::ReVar(rid) => self.resolve_var(*rid),
            _ => r,
        })
    }

    fn value(&self, rid: RegionVid) -> &VarValue<'tcx> {
        &self.values[rid]
    }

    fn value_mut(&mut self, rid: RegionVid) -> &mut VarValue<'tcx> {
        &mut self.values[rid]
    }

    pub fn resolve_var(&self, rid: RegionVid) -> ty::Region<'tcx> {
        let result = match self.values[rid] {
            VarValue::Value(r) => r,
            VarValue::ErrorValue => self.error_region,
        };
        debug!("resolve_var({:?}) = {:?}", rid, result);
        result
    }
}
