//! Liveness analysis which computes liveness of MIR local variables at the boundary of basic blocks
//!
//! This analysis considers references as being used only at the point of the
//! borrow. This means that this does not track uses because of references that
//! already exist:
//!
//! ```Rust
//!     fn foo() {
//!         x = 0;
//!         // `x` is live here
//!         GLOBAL = &x: *const u32;
//!         // but not here, even while it can be accessed through `GLOBAL`.
//!         foo();
//!         x = 1;
//!         // `x` is live again here, because it is assigned to `OTHER_GLOBAL`
//!         OTHER_GLOBAL = &x: *const u32;
//!         // ...
//!     }
//! ```
//!
//! This means that users of this analysis still have to check whether
//! pre-existing references can be used to access the value (e.g., at movable
//! generator yield points, all pre-existing references are invalidated, so this
//! doesn't matter).

use rustc::mir::visit::{
    PlaceContext, Visitor, MutatingUseContext, NonMutatingUseContext, NonUseContext,
};
use rustc::mir::Local;
use rustc::mir::*;
use rustc::ty::{item_path, TyCtxt};
use rustc_data_structures::bit_set::BitSet;
use rustc_data_structures::indexed_vec::{Idx, IndexVec};
use rustc_data_structures::work_queue::WorkQueue;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use transform::MirSource;
use util::pretty::{dump_enabled, write_basic_block, write_mir_intro};

pub type LiveVarSet<V> = BitSet<V>;

/// This gives the result of the liveness analysis at the boundary of
/// basic blocks.
///
/// The `V` type defines the set of variables that we computed
/// liveness for. This is often `Local`, in which case we computed
/// liveness for all variables -- but it can also be some other type,
/// which indicates a subset of the variables within the graph.
pub struct LivenessResult<V: Idx> {
    /// Live variables on exit to each basic block. This is equal to
    /// the union of the `ins` for each successor.
    pub outs: IndexVec<BasicBlock, LiveVarSet<V>>,
}

/// Defines the mapping to/from the MIR local variables (`Local`) to
/// the "live variable indices" we are using in a particular
/// computation.
pub trait LiveVariableMap {
    type LiveVar;

    fn from_local(&self, local: Local) -> Option<Self::LiveVar>;
    fn from_live_var(&self, local: Self::LiveVar) -> Local;
    fn num_variables(&self) -> usize;
}

#[derive(Debug)]
pub struct IdentityMap<'a, 'tcx: 'a> {
    mir: &'a Mir<'tcx>,
}

impl<'a, 'tcx> IdentityMap<'a, 'tcx> {
    pub fn new(mir: &'a Mir<'tcx>) -> Self {
        Self { mir }
    }
}

impl<'a, 'tcx> LiveVariableMap for IdentityMap<'a, 'tcx> {
    type LiveVar = Local;

    fn from_local(&self, local: Local) -> Option<Self::LiveVar> {
        Some(local)
    }

    fn from_live_var(&self, local: Self::LiveVar) -> Local {
        local
    }

    fn num_variables(&self) -> usize {
        self.mir.local_decls.len()
    }
}

/// Compute which local variables are live within the given function
/// `mir`. The liveness mode `mode` determines what sorts of uses are
/// considered to make a variable live (e.g., do drops count?).
pub fn liveness_of_locals<'tcx, V: Idx>(
    mir: &Mir<'tcx>,
    map: &impl LiveVariableMap<LiveVar = V>,
) -> LivenessResult<V> {
    let num_live_vars = map.num_variables();

    let def_use: IndexVec<_, DefsUses<V>> = mir
        .basic_blocks()
        .iter()
        .map(|b| block(map, b, num_live_vars))
        .collect();

    let mut outs: IndexVec<_, LiveVarSet<V>> = mir
        .basic_blocks()
        .indices()
        .map(|_| LiveVarSet::new_empty(num_live_vars))
        .collect();

    let mut bits = LiveVarSet::new_empty(num_live_vars);

    // queue of things that need to be re-processed, and a set containing
    // the things currently in the queue
    let mut dirty_queue: WorkQueue<BasicBlock> = WorkQueue::with_all(mir.basic_blocks().len());

    let predecessors = mir.predecessors();

    while let Some(bb) = dirty_queue.pop() {
        // bits = use ∪ (bits - def)
        bits.overwrite(&outs[bb]);
        def_use[bb].apply(&mut bits);

        // `bits` now contains the live variables on entry. Therefore,
        // add `bits` to the `out` set for each predecessor; if those
        // bits were not already present, then enqueue the predecessor
        // as dirty.
        //
        // (note that `union` returns true if the `self` set changed)
        for &pred_bb in &predecessors[bb] {
            if outs[pred_bb].union(&bits) {
                dirty_queue.insert(pred_bb);
            }
        }
    }

    LivenessResult { outs }
}

#[derive(Eq, PartialEq, Clone)]
pub enum DefUse {
    Def,
    Use,
    Drop,
}

pub fn categorize<'tcx>(context: PlaceContext<'tcx>) -> Option<DefUse> {
    match context {
        ///////////////////////////////////////////////////////////////////////////
        // DEFS

        PlaceContext::MutatingUse(MutatingUseContext::Store) |

        // This is potentially both a def and a use...
        PlaceContext::MutatingUse(MutatingUseContext::AsmOutput) |

        // We let Call define the result in both the success and
        // unwind cases. This is not really correct, however it
        // does not seem to be observable due to the way that we
        // generate MIR. To do things properly, we would apply
        // the def in call only to the input from the success
        // path and not the unwind path. -nmatsakis
        PlaceContext::MutatingUse(MutatingUseContext::Call) |

        // Storage live and storage dead aren't proper defines, but we can ignore
        // values that come before them.
        PlaceContext::NonUse(NonUseContext::StorageLive) |
        PlaceContext::NonUse(NonUseContext::StorageDead) => Some(DefUse::Def),

        ///////////////////////////////////////////////////////////////////////////
        // REGULAR USES
        //
        // These are uses that occur *outside* of a drop. For the
        // purposes of NLL, these are special in that **all** the
        // lifetimes appearing in the variable must be live for each regular use.

        PlaceContext::NonMutatingUse(NonMutatingUseContext::Projection) |
        PlaceContext::MutatingUse(MutatingUseContext::Projection) |

        // Borrows only consider their local used at the point of the borrow.
        // This won't affect the results since we use this analysis for generators
        // and we only care about the result at suspension points. Borrows cannot
        // cross suspension points so this behavior is unproblematic.
        PlaceContext::MutatingUse(MutatingUseContext::Borrow(..)) |
        PlaceContext::NonMutatingUse(NonMutatingUseContext::SharedBorrow(..)) |
        PlaceContext::NonMutatingUse(NonMutatingUseContext::ShallowBorrow(..)) |
        PlaceContext::NonMutatingUse(NonMutatingUseContext::UniqueBorrow(..)) |

        PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect) |
        PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy) |
        PlaceContext::NonMutatingUse(NonMutatingUseContext::Move) |
        PlaceContext::NonUse(NonUseContext::AscribeUserTy) |
        PlaceContext::MutatingUse(MutatingUseContext::Retag) =>
            Some(DefUse::Use),

        ///////////////////////////////////////////////////////////////////////////
        // DROP USES
        //
        // These are uses that occur in a DROP (a MIR drop, not a
        // call to `std::mem::drop()`). For the purposes of NLL,
        // uses in drop are special because `#[may_dangle]`
        // attributes can affect whether lifetimes must be live.

        PlaceContext::MutatingUse(MutatingUseContext::Drop) =>
            Some(DefUse::Drop),
    }
}

struct DefsUsesVisitor<'lv, V, M>
where
    V: Idx,
    M: LiveVariableMap<LiveVar = V> + 'lv,
{
    map: &'lv M,
    defs_uses: DefsUses<V>,
}

#[derive(Eq, PartialEq, Clone)]
struct DefsUses<V: Idx> {
    defs: LiveVarSet<V>,
    uses: LiveVarSet<V>,
}

impl<V: Idx> DefsUses<V> {
    fn apply(&self, bits: &mut LiveVarSet<V>) -> bool {
        bits.subtract(&self.defs) | bits.union(&self.uses)
    }

    fn add_def(&mut self, index: V) {
        // If it was used already in the block, remove that use
        // now that we found a definition.
        //
        // Example:
        //
        //     // Defs = {X}, Uses = {}
        //     X = 5
        //     // Defs = {}, Uses = {X}
        //     use(X)
        self.uses.remove(index);
        self.defs.insert(index);
    }

    fn add_use(&mut self, index: V) {
        // Inverse of above.
        //
        // Example:
        //
        //     // Defs = {}, Uses = {X}
        //     use(X)
        //     // Defs = {X}, Uses = {}
        //     X = 5
        //     // Defs = {}, Uses = {X}
        //     use(X)
        self.defs.remove(index);
        self.uses.insert(index);
    }
}

impl<'tcx, 'lv, V, M> Visitor<'tcx> for DefsUsesVisitor<'lv, V, M>
where
    V: Idx,
    M: LiveVariableMap<LiveVar = V>,
{
    fn visit_local(&mut self, &local: &Local, context: PlaceContext<'tcx>, _: Location) {
        if let Some(v_index) = self.map.from_local(local) {
            match categorize(context) {
                Some(DefUse::Def) => self.defs_uses.add_def(v_index),
                Some(DefUse::Use) | Some(DefUse::Drop) => self.defs_uses.add_use(v_index),
                _ => (),
            }
        }
    }
}

fn block<'tcx, V: Idx>(
    map: &impl LiveVariableMap<LiveVar = V>,
    b: &BasicBlockData<'tcx>,
    locals: usize,
) -> DefsUses<V> {
    let mut visitor = DefsUsesVisitor {
        map,
        defs_uses: DefsUses {
            defs: LiveVarSet::new_empty(locals),
            uses: LiveVarSet::new_empty(locals),
        },
    };

    let dummy_location = Location {
        block: BasicBlock::new(0),
        statement_index: 0,
    };

    // Visit the various parts of the basic block in reverse. If we go
    // forward, the logic in `add_def` and `add_use` would be wrong.
    visitor.visit_terminator(BasicBlock::new(0), b.terminator(), dummy_location);
    for statement in b.statements.iter().rev() {
        visitor.visit_statement(BasicBlock::new(0), statement, dummy_location);
    }

    visitor.defs_uses
}

pub fn dump_mir<'a, 'tcx, V: Idx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    pass_name: &str,
    source: MirSource,
    mir: &Mir<'tcx>,
    map: &impl LiveVariableMap<LiveVar = V>,
    result: &LivenessResult<V>,
) {
    if !dump_enabled(tcx, pass_name, source) {
        return;
    }
    let node_path = item_path::with_forced_impl_filename_line(|| {
        // see notes on #41697 below
        tcx.item_path_str(source.def_id)
    });
    dump_matched_mir_node(tcx, pass_name, &node_path, source, mir, map, result);
}

fn dump_matched_mir_node<'a, 'tcx, V: Idx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    pass_name: &str,
    node_path: &str,
    source: MirSource,
    mir: &Mir<'tcx>,
    map: &dyn LiveVariableMap<LiveVar = V>,
    result: &LivenessResult<V>,
) {
    let mut file_path = PathBuf::new();
    file_path.push(Path::new(&tcx.sess.opts.debugging_opts.dump_mir_dir));
    let item_id = tcx.hir().as_local_node_id(source.def_id).unwrap();
    let file_name = format!("rustc.node{}{}-liveness.mir", item_id, pass_name);
    file_path.push(&file_name);
    let _ = fs::File::create(&file_path).and_then(|mut file| {
        writeln!(file, "// MIR local liveness analysis for `{}`", node_path)?;
        writeln!(file, "// source = {:?}", source)?;
        writeln!(file, "// pass_name = {}", pass_name)?;
        writeln!(file, "")?;
        write_mir_fn(tcx, source, mir, map, &mut file, result)?;
        Ok(())
    });
}

pub fn write_mir_fn<'a, 'tcx, V: Idx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    src: MirSource,
    mir: &Mir<'tcx>,
    map: &dyn LiveVariableMap<LiveVar = V>,
    w: &mut dyn Write,
    result: &LivenessResult<V>,
) -> io::Result<()> {
    write_mir_intro(tcx, src, mir, w)?;
    for block in mir.basic_blocks().indices() {
        let print = |w: &mut dyn Write, prefix, result: &IndexVec<BasicBlock, LiveVarSet<V>>| {
            let live: Vec<String> = result[block]
                .iter()
                .map(|v| map.from_live_var(v))
                .map(|local| format!("{:?}", local))
                .collect();
            writeln!(w, "{} {{{}}}", prefix, live.join(", "))
        };
        write_basic_block(tcx, block, mir, &mut |_, _| Ok(()), w)?;
        print(w, "   ", &result.outs)?;
        if block.index() + 1 != mir.basic_blocks().len() {
            writeln!(w, "")?;
        }
    }

    writeln!(w, "}}")?;
    Ok(())
}
