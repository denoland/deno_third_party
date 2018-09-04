// Copyright 2012-2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use syntax::ast::{self, MetaItem};

use rustc_data_structures::indexed_set::{IdxSet, IdxSetBuf};
use rustc_data_structures::indexed_vec::Idx;
use rustc_data_structures::bitslice::{bitwise, BitwiseOperator};

use rustc::ty::{self, TyCtxt};
use rustc::mir::{self, Mir, BasicBlock, BasicBlockData, Location, Statement, Terminator};
use rustc::session::Session;

use std::borrow::Borrow;
use std::fmt;
use std::io;
use std::mem;
use std::path::PathBuf;
use std::usize;

pub use self::impls::{MaybeStorageLive};
pub use self::impls::{MaybeInitializedPlaces, MaybeUninitializedPlaces};
pub use self::impls::{DefinitelyInitializedPlaces, MovingOutStatements};
pub use self::impls::EverInitializedPlaces;
pub use self::impls::borrows::Borrows;
pub use self::impls::HaveBeenBorrowedLocals;
pub use self::at_location::{FlowAtLocation, FlowsAtLocation};
pub(crate) use self::drop_flag_effects::*;

use self::move_paths::MoveData;

mod at_location;
mod drop_flag_effects;
mod graphviz;
mod impls;
pub mod move_paths;

pub(crate) use self::move_paths::indexes;

pub(crate) struct DataflowBuilder<'a, 'tcx: 'a, BD> where BD: BitDenotation
{
    node_id: ast::NodeId,
    flow_state: DataflowAnalysis<'a, 'tcx, BD>,
    print_preflow_to: Option<String>,
    print_postflow_to: Option<String>,
}

/// `DebugFormatted` encapsulates the "{:?}" rendering of some
/// arbitrary value. This way: you pay cost of allocating an extra
/// string (as well as that of rendering up-front); in exchange, you
/// don't have to hand over ownership of your value or deal with
/// borrowing it.
pub(crate) struct DebugFormatted(String);

impl DebugFormatted {
    pub fn new(input: &dyn fmt::Debug) -> DebugFormatted {
        DebugFormatted(format!("{:?}", input))
    }
}

impl fmt::Debug for DebugFormatted {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        write!(w, "{}", self.0)
    }
}

pub(crate) trait Dataflow<BD: BitDenotation> {
    /// Sets up and runs the dataflow problem, using `p` to render results if
    /// implementation so chooses.
    fn dataflow<P>(&mut self, p: P) where P: Fn(&BD, BD::Idx) -> DebugFormatted {
        let _ = p; // default implementation does not instrument process.
        self.build_sets();
        self.propagate();
    }

    /// Sets up the entry, gen, and kill sets for this instance of a dataflow problem.
    fn build_sets(&mut self);

    /// Finds a fixed-point solution to this instance of a dataflow problem.
    fn propagate(&mut self);
}

impl<'a, 'tcx: 'a, BD> Dataflow<BD> for DataflowBuilder<'a, 'tcx, BD> where BD: BitDenotation
{
    fn dataflow<P>(&mut self, p: P) where P: Fn(&BD, BD::Idx) -> DebugFormatted {
        self.flow_state.build_sets();
        self.pre_dataflow_instrumentation(|c,i| p(c,i)).unwrap();
        self.flow_state.propagate();
        self.post_dataflow_instrumentation(|c,i| p(c,i)).unwrap();
    }

    fn build_sets(&mut self) { self.flow_state.build_sets(); }
    fn propagate(&mut self) { self.flow_state.propagate(); }
}

pub(crate) fn has_rustc_mir_with(attrs: &[ast::Attribute], name: &str) -> Option<MetaItem> {
    for attr in attrs {
        if attr.check_name("rustc_mir") {
            let items = attr.meta_item_list();
            for item in items.iter().flat_map(|l| l.iter()) {
                match item.meta_item() {
                    Some(mi) if mi.check_name(name) => return Some(mi.clone()),
                    _ => continue
                }
            }
        }
    }
    return None;
}

pub struct MoveDataParamEnv<'gcx, 'tcx> {
    pub(crate) move_data: MoveData<'tcx>,
    pub(crate) param_env: ty::ParamEnv<'gcx>,
}

pub(crate) fn do_dataflow<'a, 'gcx, 'tcx, BD, P>(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                                 mir: &'a Mir<'tcx>,
                                                 node_id: ast::NodeId,
                                                 attributes: &[ast::Attribute],
                                                 dead_unwinds: &IdxSet<BasicBlock>,
                                                 bd: BD,
                                                 p: P)
                                                 -> DataflowResults<BD>
    where BD: BitDenotation + InitialFlow,
          P: Fn(&BD, BD::Idx) -> DebugFormatted
{
    let flow_state = DataflowAnalysis::new(mir, dead_unwinds, bd);
    flow_state.run(tcx, node_id, attributes, p)
}

impl<'a, 'gcx: 'tcx, 'tcx: 'a, BD> DataflowAnalysis<'a, 'tcx, BD> where BD: BitDenotation
{
    pub(crate) fn run<P>(self,
                         tcx: TyCtxt<'a, 'gcx, 'tcx>,
                         node_id: ast::NodeId,
                         attributes: &[ast::Attribute],
                         p: P) -> DataflowResults<BD>
        where P: Fn(&BD, BD::Idx) -> DebugFormatted
    {
        let name_found = |sess: &Session, attrs: &[ast::Attribute], name| -> Option<String> {
            if let Some(item) = has_rustc_mir_with(attrs, name) {
                if let Some(s) = item.value_str() {
                    return Some(s.to_string())
                } else {
                    sess.span_err(
                        item.span,
                        &format!("{} attribute requires a path", item.ident));
                    return None;
                }
            }
            return None;
        };

        let print_preflow_to =
            name_found(tcx.sess, attributes, "borrowck_graphviz_preflow");
        let print_postflow_to =
            name_found(tcx.sess, attributes, "borrowck_graphviz_postflow");

        let mut mbcx = DataflowBuilder {
            node_id,
            print_preflow_to, print_postflow_to, flow_state: self,
        };

        mbcx.dataflow(p);
        mbcx.flow_state.results()
    }
}

struct PropagationContext<'b, 'a: 'b, 'tcx: 'a, O> where O: 'b + BitDenotation
{
    builder: &'b mut DataflowAnalysis<'a, 'tcx, O>,
    changed: bool,
}

impl<'a, 'tcx: 'a, BD> DataflowAnalysis<'a, 'tcx, BD> where BD: BitDenotation
{
    fn propagate(&mut self) {
        let mut temp = IdxSetBuf::new_empty(self.flow_state.sets.bits_per_block);
        let mut propcx = PropagationContext {
            builder: self,
            changed: true,
        };
        while propcx.changed {
            propcx.changed = false;
            propcx.walk_cfg(&mut temp);
        }
    }

    fn build_sets(&mut self) {
        // First we need to build the entry-, gen- and kill-sets.

        {
            let sets = &mut self.flow_state.sets.for_block(mir::START_BLOCK.index());
            self.flow_state.operator.start_block_effect(&mut sets.on_entry);
        }

        for (bb, data) in self.mir.basic_blocks().iter_enumerated() {
            let &mir::BasicBlockData { ref statements, ref terminator, is_cleanup: _ } = data;

            let mut interim_state;
            let sets = &mut self.flow_state.sets.for_block(bb.index());
            let track_intrablock = BD::accumulates_intrablock_state();
            if track_intrablock {
                debug!("swapping in mutable on_entry, initially {:?}", sets.on_entry);
                interim_state = sets.on_entry.to_owned();
                sets.on_entry = &mut interim_state;
            }
            for j_stmt in 0..statements.len() {
                let location = Location { block: bb, statement_index: j_stmt };
                self.flow_state.operator.before_statement_effect(sets, location);
                self.flow_state.operator.statement_effect(sets, location);
                if track_intrablock {
                    sets.apply_local_effect();
                }
            }

            if terminator.is_some() {
                let location = Location { block: bb, statement_index: statements.len() };
                self.flow_state.operator.before_terminator_effect(sets, location);
                self.flow_state.operator.terminator_effect(sets, location);
                if track_intrablock {
                    sets.apply_local_effect();
                }
            }
        }
    }
}

impl<'b, 'a: 'b, 'tcx: 'a, BD> PropagationContext<'b, 'a, 'tcx, BD> where BD: BitDenotation
{
    fn walk_cfg(&mut self, in_out: &mut IdxSet<BD::Idx>) {
        let mir = self.builder.mir;
        for (bb_idx, bb_data) in mir.basic_blocks().iter().enumerate() {
            let builder = &mut self.builder;
            {
                let sets = builder.flow_state.sets.for_block(bb_idx);
                debug_assert!(in_out.words().len() == sets.on_entry.words().len());
                in_out.clone_from(sets.on_entry);
                in_out.union(sets.gen_set);
                in_out.subtract(sets.kill_set);
            }
            builder.propagate_bits_into_graph_successors_of(
                in_out, &mut self.changed, (mir::BasicBlock::new(bb_idx), bb_data));
        }
    }
}

fn dataflow_path(context: &str, prepost: &str, path: &str) -> PathBuf {
    format!("{}_{}", context, prepost);
    let mut path = PathBuf::from(path);
    let new_file_name = {
        let orig_file_name = path.file_name().unwrap().to_str().unwrap();
        format!("{}_{}", context, orig_file_name)
    };
    path.set_file_name(new_file_name);
    path
}

impl<'a, 'tcx: 'a, BD> DataflowBuilder<'a, 'tcx, BD> where BD: BitDenotation
{
    fn pre_dataflow_instrumentation<P>(&self, p: P) -> io::Result<()>
        where P: Fn(&BD, BD::Idx) -> DebugFormatted
    {
        if let Some(ref path_str) = self.print_preflow_to {
            let path = dataflow_path(BD::name(), "preflow", path_str);
            graphviz::print_borrowck_graph_to(self, &path, p)
        } else {
            Ok(())
        }
    }

    fn post_dataflow_instrumentation<P>(&self, p: P) -> io::Result<()>
        where P: Fn(&BD, BD::Idx) -> DebugFormatted
    {
        if let Some(ref path_str) = self.print_postflow_to {
            let path = dataflow_path(BD::name(), "postflow", path_str);
            graphviz::print_borrowck_graph_to(self, &path, p)
        } else{
            Ok(())
        }
    }
}

/// Maps each block to a set of bits
#[derive(Debug)]
pub(crate) struct Bits<E:Idx> {
    bits: IdxSetBuf<E>,
}

impl<E:Idx> Clone for Bits<E> {
    fn clone(&self) -> Self { Bits { bits: self.bits.clone() } }
}

impl<E:Idx> Bits<E> {
    fn new(bits: IdxSetBuf<E>) -> Self {
        Bits { bits: bits }
    }
}

/// DataflowResultsConsumer abstracts over walking the MIR with some
/// already constructed dataflow results.
///
/// It abstracts over the FlowState and also completely hides the
/// underlying flow analysis results, because it needs to handle cases
/// where we are combining the results of *multiple* flow analyses
/// (e.g. borrows + inits + uninits).
pub(crate) trait DataflowResultsConsumer<'a, 'tcx: 'a> {
    type FlowState: FlowsAtLocation;

    // Observation Hooks: override (at least one of) these to get analysis feedback.
    fn visit_block_entry(&mut self,
                         _bb: BasicBlock,
                         _flow_state: &Self::FlowState) {}

    fn visit_statement_entry(&mut self,
                             _loc: Location,
                             _stmt: &Statement<'tcx>,
                             _flow_state: &Self::FlowState) {}

    fn visit_terminator_entry(&mut self,
                              _loc: Location,
                              _term: &Terminator<'tcx>,
                              _flow_state: &Self::FlowState) {}

    // Main entry point: this drives the processing of results.

    fn analyze_results(&mut self, flow_uninit: &mut Self::FlowState) {
        let flow = flow_uninit;
        for bb in self.mir().basic_blocks().indices() {
            flow.reset_to_entry_of(bb);
            self.process_basic_block(bb, flow);
        }
    }

    fn process_basic_block(&mut self, bb: BasicBlock, flow_state: &mut Self::FlowState) {
        let BasicBlockData { ref statements, ref terminator, is_cleanup: _ } =
            self.mir()[bb];
        let mut location = Location { block: bb, statement_index: 0 };
        for stmt in statements.iter() {
            flow_state.reconstruct_statement_effect(location);
            self.visit_statement_entry(location, stmt, flow_state);
            flow_state.apply_local_effect(location);
            location.statement_index += 1;
        }

        if let Some(ref term) = *terminator {
            flow_state.reconstruct_terminator_effect(location);
            self.visit_terminator_entry(location, term, flow_state);

            // We don't need to apply the effect of the terminator,
            // since we are only visiting dataflow state on control
            // flow entry to the various nodes. (But we still need to
            // reconstruct the effect, because the visit method might
            // inspect it.)
        }
    }

    // Delegated Hooks: Provide access to the MIR and process the flow state.

    fn mir(&self) -> &'a Mir<'tcx>;
}

pub fn state_for_location<'tcx, T: BitDenotation>(loc: Location,
                                                  analysis: &T,
                                                  result: &DataflowResults<T>,
                                                  mir: &Mir<'tcx>)
    -> IdxSetBuf<T::Idx> {
    let mut entry = result.sets().on_entry_set_for(loc.block.index()).to_owned();

    {
        let mut sets = BlockSets {
            on_entry: &mut entry.clone(),
            kill_set: &mut entry.clone(),
            gen_set: &mut entry,
        };

        for stmt in 0..loc.statement_index {
            let mut stmt_loc = loc;
            stmt_loc.statement_index = stmt;
            analysis.before_statement_effect(&mut sets, stmt_loc);
            analysis.statement_effect(&mut sets, stmt_loc);
        }

        // Apply the pre-statement effect of the statement we're evaluating.
        if loc.statement_index == mir[loc.block].statements.len() {
            analysis.before_terminator_effect(&mut sets, loc);
        } else {
            analysis.before_statement_effect(&mut sets, loc);
        }
    }

    entry
}

pub struct DataflowAnalysis<'a, 'tcx: 'a, O> where O: BitDenotation
{
    flow_state: DataflowState<O>,
    dead_unwinds: &'a IdxSet<mir::BasicBlock>,
    mir: &'a Mir<'tcx>,
}

impl<'a, 'tcx: 'a, O> DataflowAnalysis<'a, 'tcx, O> where O: BitDenotation
{
    pub fn results(self) -> DataflowResults<O> {
        DataflowResults(self.flow_state)
    }

    pub fn mir(&self) -> &'a Mir<'tcx> { self.mir }
}

pub struct DataflowResults<O>(pub(crate) DataflowState<O>) where O: BitDenotation;

impl<O: BitDenotation> DataflowResults<O> {
    pub fn sets(&self) -> &AllSets<O::Idx> {
        &self.0.sets
    }

    pub fn operator(&self) -> &O {
        &self.0.operator
    }
}

/// State of a dataflow analysis; couples a collection of bit sets
/// with operator used to initialize and merge bits during analysis.
pub struct DataflowState<O: BitDenotation>
{
    /// All the sets for the analysis. (Factored into its
    /// own structure so that we can borrow it mutably
    /// on its own separate from other fields.)
    pub sets: AllSets<O::Idx>,

    /// operator used to initialize, combine, and interpret bits.
    pub(crate) operator: O,
}

impl<O: BitDenotation> DataflowState<O> {
    pub fn each_bit<F>(&self, words: &IdxSet<O::Idx>, f: F) where F: FnMut(O::Idx)
    {
        words.iter().for_each(f)
    }

    pub(crate) fn interpret_set<'c, P>(&self,
                                       o: &'c O,
                                       words: &IdxSet<O::Idx>,
                                       render_idx: &P)
                                       -> Vec<DebugFormatted>
        where P: Fn(&O, O::Idx) -> DebugFormatted
    {
        let mut v = Vec::new();
        self.each_bit(words, |i| {
            v.push(render_idx(o, i));
        });
        v
    }
}

#[derive(Debug)]
pub struct AllSets<E: Idx> {
    /// Analysis bitwidth for each block.
    bits_per_block: usize,

    /// Number of words associated with each block entry
    /// equal to bits_per_block / usize::BITS, rounded up.
    words_per_block: usize,

    /// For each block, bits generated by executing the statements in
    /// the block. (For comparison, the Terminator for each block is
    /// handled in a flow-specific manner during propagation.)
    gen_sets: Bits<E>,

    /// For each block, bits killed by executing the statements in the
    /// block. (For comparison, the Terminator for each block is
    /// handled in a flow-specific manner during propagation.)
    kill_sets: Bits<E>,

    /// For each block, bits valid on entry to the block.
    on_entry_sets: Bits<E>,
}

/// Triple of sets associated with a given block.
///
/// Generally, one sets up `on_entry`, `gen_set`, and `kill_set` for
/// each block individually, and then runs the dataflow analysis which
/// iteratively modifies the various `on_entry` sets (but leaves the
/// other two sets unchanged, since they represent the effect of the
/// block, which should be invariant over the course of the analysis).
///
/// It is best to ensure that the intersection of `gen_set` and
/// `kill_set` is empty; otherwise the results of the dataflow will
/// have a hidden dependency on what order the bits are generated and
/// killed during the iteration. (This is such a good idea that the
/// `fn gen` and `fn kill` methods that set their state enforce this
/// for you.)
#[derive(Debug)]
pub struct BlockSets<'a, E: Idx> {
    /// Dataflow state immediately before control flow enters the given block.
    pub(crate) on_entry: &'a mut IdxSet<E>,

    /// Bits that are set to 1 by the time we exit the given block.
    pub(crate) gen_set: &'a mut IdxSet<E>,

    /// Bits that are set to 0 by the time we exit the given block.
    pub(crate) kill_set: &'a mut IdxSet<E>,
}

impl<'a, E:Idx> BlockSets<'a, E> {
    fn gen(&mut self, e: &E) {
        self.gen_set.add(e);
        self.kill_set.remove(e);
    }
    fn gen_all<I>(&mut self, i: I)
        where I: IntoIterator,
              I::Item: Borrow<E>
    {
        for j in i {
            self.gen(j.borrow());
        }
    }

    fn gen_all_and_assert_dead<I>(&mut self, i: I)
        where I: IntoIterator,
        I::Item: Borrow<E>
    {
        for j in i {
            let j = j.borrow();
            let retval = self.gen_set.add(j);
            self.kill_set.remove(j);
            assert!(retval);
        }
    }

    fn kill(&mut self, e: &E) {
        self.gen_set.remove(e);
        self.kill_set.add(e);
    }

    fn kill_all<I>(&mut self, i: I)
        where I: IntoIterator,
              I::Item: Borrow<E>
    {
        for j in i {
            self.kill(j.borrow());
        }
    }

    fn apply_local_effect(&mut self) {
        self.on_entry.union(&self.gen_set);
        self.on_entry.subtract(&self.kill_set);
    }
}

impl<E:Idx> AllSets<E> {
    pub fn bits_per_block(&self) -> usize { self.bits_per_block }
    pub fn for_block(&mut self, block_idx: usize) -> BlockSets<E> {
        let offset = self.words_per_block * block_idx;
        let range = E::new(offset)..E::new(offset + self.words_per_block);
        BlockSets {
            on_entry: self.on_entry_sets.bits.range_mut(&range),
            gen_set: self.gen_sets.bits.range_mut(&range),
            kill_set: self.kill_sets.bits.range_mut(&range),
        }
    }

    fn lookup_set_for<'a>(&self, sets: &'a Bits<E>, block_idx: usize) -> &'a IdxSet<E> {
        let offset = self.words_per_block * block_idx;
        let range = E::new(offset)..E::new(offset + self.words_per_block);
        sets.bits.range(&range)
    }
    pub fn gen_set_for(&self, block_idx: usize) -> &IdxSet<E> {
        self.lookup_set_for(&self.gen_sets, block_idx)
    }
    pub fn kill_set_for(&self, block_idx: usize) -> &IdxSet<E> {
        self.lookup_set_for(&self.kill_sets, block_idx)
    }
    pub fn on_entry_set_for(&self, block_idx: usize) -> &IdxSet<E> {
        self.lookup_set_for(&self.on_entry_sets, block_idx)
    }
}

/// Parameterization for the precise form of data flow that is used.
/// `InitialFlow` handles initializing the bitvectors before any
/// code is inspected by the analysis. Analyses that need more nuanced
/// initialization (e.g. they need to consult the results of some other
/// dataflow analysis to set up the initial bitvectors) should not
/// implement this.
pub trait InitialFlow {
    /// Specifies the initial value for each bit in the `on_entry` set
    fn bottom_value() -> bool;
}

pub trait BitDenotation: BitwiseOperator {
    /// Specifies what index type is used to access the bitvector.
    type Idx: Idx;

    /// Some analyses want to accumulate knowledge within a block when
    /// analyzing its statements for building the gen/kill sets. Override
    /// this method to return true in such cases.
    ///
    /// When this returns true, the statement-effect (re)construction
    /// will clone the `on_entry` state and pass along a reference via
    /// `sets.on_entry` to that local clone into `statement_effect` and
    /// `terminator_effect`).
    ///
    /// When its false, no local clone is constucted; instead a
    /// reference directly into `on_entry` is passed along via
    /// `sets.on_entry` instead, which represents the flow state at
    /// the block's start, not necessarily the state immediately prior
    /// to the statement/terminator under analysis.
    ///
    /// In either case, the passed reference is mutable; but this is a
    /// wart from using the `BlockSets` type in the API; the intention
    /// is that the `statement_effect` and `terminator_effect` methods
    /// mutate only the gen/kill sets.
    ///
    /// FIXME: We should consider enforcing the intention described in
    /// the previous paragraph by passing the three sets in separate
    /// parameters to encode their distinct mutabilities.
    fn accumulates_intrablock_state() -> bool { false }

    /// A name describing the dataflow analysis that this
    /// BitDenotation is supporting.  The name should be something
    /// suitable for plugging in as part of a filename e.g. avoid
    /// space-characters or other things that tend to look bad on a
    /// file system, like slashes or periods. It is also better for
    /// the name to be reasonably short, again because it will be
    /// plugged into a filename.
    fn name() -> &'static str;

    /// Size of each bitvector allocated for each block in the analysis.
    fn bits_per_block(&self) -> usize;

    /// Mutates the entry set according to the effects that
    /// have been established *prior* to entering the start
    /// block. This can't access the gen/kill sets, because
    /// these won't be accounted for correctly.
    ///
    /// (For example, establishing the call arguments.)
    fn start_block_effect(&self, entry_set: &mut IdxSet<Self::Idx>);

    /// Similar to `statement_effect`, except it applies
    /// *just before* the statement rather than *just after* it.
    ///
    /// This matters for "dataflow at location" APIs, because the
    /// before-statement effect is visible while visiting the
    /// statement, while the after-statement effect only becomes
    /// visible at the next statement.
    ///
    /// Both the before-statement and after-statement effects are
    /// applied, in that order, before moving for the next
    /// statement.
    fn before_statement_effect(&self,
                               _sets: &mut BlockSets<Self::Idx>,
                               _location: Location) {}

    /// Mutates the block-sets (the flow sets for the given
    /// basic block) according to the effects of evaluating statement.
    ///
    /// This is used, in particular, for building up the
    /// "transfer-function" representing the overall-effect of the
    /// block, represented via GEN and KILL sets.
    ///
    /// The statement is identified as `bb_data[idx_stmt]`, where
    /// `bb_data` is the sequence of statements identified by `bb` in
    /// the MIR.
    fn statement_effect(&self,
                        sets: &mut BlockSets<Self::Idx>,
                        location: Location);

    /// Similar to `terminator_effect`, except it applies
    /// *just before* the terminator rather than *just after* it.
    ///
    /// This matters for "dataflow at location" APIs, because the
    /// before-terminator effect is visible while visiting the
    /// terminator, while the after-terminator effect only becomes
    /// visible at the terminator's successors.
    ///
    /// Both the before-terminator and after-terminator effects are
    /// applied, in that order, before moving for the next
    /// terminator.
    fn before_terminator_effect(&self,
                                _sets: &mut BlockSets<Self::Idx>,
                                _location: Location) {}

    /// Mutates the block-sets (the flow sets for the given
    /// basic block) according to the effects of evaluating
    /// the terminator.
    ///
    /// This is used, in particular, for building up the
    /// "transfer-function" representing the overall-effect of the
    /// block, represented via GEN and KILL sets.
    ///
    /// The effects applied here cannot depend on which branch the
    /// terminator took.
    fn terminator_effect(&self,
                         sets: &mut BlockSets<Self::Idx>,
                         location: Location);

    /// Mutates the block-sets according to the (flow-dependent)
    /// effect of a successful return from a Call terminator.
    ///
    /// If basic-block BB_x ends with a call-instruction that, upon
    /// successful return, flows to BB_y, then this method will be
    /// called on the exit flow-state of BB_x in order to set up the
    /// entry flow-state of BB_y.
    ///
    /// This is used, in particular, as a special case during the
    /// "propagate" loop where all of the basic blocks are repeatedly
    /// visited. Since the effects of a Call terminator are
    /// flow-dependent, the current MIR cannot encode them via just
    /// GEN and KILL sets attached to the block, and so instead we add
    /// this extra machinery to represent the flow-dependent effect.
    ///
    /// FIXME: Right now this is a bit of a wart in the API. It might
    /// be better to represent this as an additional gen- and
    /// kill-sets associated with each edge coming out of the basic
    /// block.
    fn propagate_call_return(&self,
                             in_out: &mut IdxSet<Self::Idx>,
                             call_bb: mir::BasicBlock,
                             dest_bb: mir::BasicBlock,
                             dest_place: &mir::Place);
}

impl<'a, 'tcx, D> DataflowAnalysis<'a, 'tcx, D> where D: BitDenotation
{
    pub fn new(mir: &'a Mir<'tcx>,
               dead_unwinds: &'a IdxSet<mir::BasicBlock>,
               denotation: D) -> Self where D: InitialFlow {
        let bits_per_block = denotation.bits_per_block();
        let usize_bits = mem::size_of::<usize>() * 8;
        let words_per_block = (bits_per_block + usize_bits - 1) / usize_bits;
        let num_overall = Self::num_bits_overall(mir, bits_per_block);

        let zeroes = Bits::new(IdxSetBuf::new_empty(num_overall));
        let on_entry = Bits::new(if D::bottom_value() {
            IdxSetBuf::new_filled(num_overall)
        } else {
            IdxSetBuf::new_empty(num_overall)
        });

        DataflowAnalysis {
            mir,
            dead_unwinds,
            flow_state: DataflowState {
                sets: AllSets {
                    bits_per_block,
                    words_per_block,
                    gen_sets: zeroes.clone(),
                    kill_sets: zeroes,
                    on_entry_sets: on_entry,
                },
                operator: denotation,
            }
        }
    }

    pub fn new_from_sets(mir: &'a Mir<'tcx>,
                         dead_unwinds: &'a IdxSet<mir::BasicBlock>,
                         sets: AllSets<D::Idx>,
                         denotation: D) -> Self {
        DataflowAnalysis {
            mir,
            dead_unwinds,
            flow_state: DataflowState {
                sets: sets,
                operator: denotation,
            }
        }
    }

    fn num_bits_overall(mir: &Mir, bits_per_block: usize) -> usize {
        let usize_bits = mem::size_of::<usize>() * 8;
        let words_per_block = (bits_per_block + usize_bits - 1) / usize_bits;

        // (now rounded up to multiple of word size)
        let bits_per_block = words_per_block * usize_bits;

        let num_blocks = mir.basic_blocks().len();
        let num_overall = num_blocks * bits_per_block;
        num_overall
    }
}

impl<'a, 'tcx: 'a, D> DataflowAnalysis<'a, 'tcx, D> where D: BitDenotation
{
    /// Propagates the bits of `in_out` into all the successors of `bb`,
    /// using bitwise operator denoted by `self.operator`.
    ///
    /// For most blocks, this is entirely uniform. However, for blocks
    /// that end with a call terminator, the effect of the call on the
    /// dataflow state may depend on whether the call returned
    /// successfully or unwound.
    ///
    /// To reflect this, the `propagate_call_return` method of the
    /// `BitDenotation` mutates `in_out` when propagating `in_out` via
    /// a call terminator; such mutation is performed *last*, to
    /// ensure its side-effects do not leak elsewhere (e.g. into
    /// unwind target).
    fn propagate_bits_into_graph_successors_of(
        &mut self,
        in_out: &mut IdxSet<D::Idx>,
        changed: &mut bool,
        (bb, bb_data): (mir::BasicBlock, &mir::BasicBlockData))
    {
        match bb_data.terminator().kind {
            mir::TerminatorKind::Return |
            mir::TerminatorKind::Resume |
            mir::TerminatorKind::Abort |
            mir::TerminatorKind::GeneratorDrop |
            mir::TerminatorKind::Unreachable => {}
            mir::TerminatorKind::Goto { ref target } |
            mir::TerminatorKind::Assert { ref target, cleanup: None, .. } |
            mir::TerminatorKind::Yield { resume: ref target, drop: None, .. } |
            mir::TerminatorKind::Drop { ref target, location: _, unwind: None } |
            mir::TerminatorKind::DropAndReplace {
                ref target, value: _, location: _, unwind: None
            } => {
                self.propagate_bits_into_entry_set_for(in_out, changed, target);
            }
            mir::TerminatorKind::Yield { resume: ref target, drop: Some(ref drop), .. } => {
                self.propagate_bits_into_entry_set_for(in_out, changed, target);
                self.propagate_bits_into_entry_set_for(in_out, changed, drop);
            }
            mir::TerminatorKind::Assert { ref target, cleanup: Some(ref unwind), .. } |
            mir::TerminatorKind::Drop { ref target, location: _, unwind: Some(ref unwind) } |
            mir::TerminatorKind::DropAndReplace {
                ref target, value: _, location: _, unwind: Some(ref unwind)
            } => {
                self.propagate_bits_into_entry_set_for(in_out, changed, target);
                if !self.dead_unwinds.contains(&bb) {
                    self.propagate_bits_into_entry_set_for(in_out, changed, unwind);
                }
            }
            mir::TerminatorKind::SwitchInt { ref targets, .. } => {
                for target in targets {
                    self.propagate_bits_into_entry_set_for(in_out, changed, target);
                }
            }
            mir::TerminatorKind::Call { ref cleanup, ref destination, func: _, args: _ } => {
                if let Some(ref unwind) = *cleanup {
                    if !self.dead_unwinds.contains(&bb) {
                        self.propagate_bits_into_entry_set_for(in_out, changed, unwind);
                    }
                }
                if let Some((ref dest_place, ref dest_bb)) = *destination {
                    // N.B.: This must be done *last*, after all other
                    // propagation, as documented in comment above.
                    self.flow_state.operator.propagate_call_return(
                        in_out, bb, *dest_bb, dest_place);
                    self.propagate_bits_into_entry_set_for(in_out, changed, dest_bb);
                }
            }
            mir::TerminatorKind::FalseEdges { ref real_target, ref imaginary_targets } => {
                self.propagate_bits_into_entry_set_for(in_out, changed, real_target);
                for target in imaginary_targets {
                    self.propagate_bits_into_entry_set_for(in_out, changed, target);
                }
            }
            mir::TerminatorKind::FalseUnwind { ref real_target, unwind } => {
                self.propagate_bits_into_entry_set_for(in_out, changed, real_target);
                if let Some(ref unwind) = unwind {
                    if !self.dead_unwinds.contains(&bb) {
                        self.propagate_bits_into_entry_set_for(in_out, changed, unwind);
                    }
                }
            }
        }
    }

    fn propagate_bits_into_entry_set_for(&mut self,
                                         in_out: &IdxSet<D::Idx>,
                                         changed: &mut bool,
                                         bb: &mir::BasicBlock) {
        let entry_set = self.flow_state.sets.for_block(bb.index()).on_entry;
        let set_changed = bitwise(entry_set.words_mut(),
                                  in_out.words(),
                                  &self.flow_state.operator);
        if set_changed {
            *changed = true;
        }
    }
}
