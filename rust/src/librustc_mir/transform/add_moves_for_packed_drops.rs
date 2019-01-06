use rustc::hir::def_id::DefId;
use rustc::mir::*;
use rustc::ty::TyCtxt;

use transform::{MirPass, MirSource};
use util::patch::MirPatch;
use util;

// This pass moves values being dropped that are within a packed
// struct to a separate local before dropping them, to ensure that
// they are dropped from an aligned address.
//
// For example, if we have something like
// ```Rust
//     #[repr(packed)]
//     struct Foo {
//         dealign: u8,
//         data: Vec<u8>
//     }
//
//     let foo = ...;
// ```
//
// We want to call `drop_in_place::<Vec<u8>>` on `data` from an aligned
// address. This means we can't simply drop `foo.data` directly, because
// its address is not aligned.
//
// Instead, we move `foo.data` to a local and drop that:
// ```
//     storage.live(drop_temp)
//     drop_temp = foo.data;
//     drop(drop_temp) -> next
// next:
//     storage.dead(drop_temp)
// ```
//
// The storage instructions are required to avoid stack space
// blowup.

pub struct AddMovesForPackedDrops;

impl MirPass for AddMovesForPackedDrops {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          src: MirSource,
                          mir: &mut Mir<'tcx>)
    {
        debug!("add_moves_for_packed_drops({:?} @ {:?})", src, mir.span);
        add_moves_for_packed_drops(tcx, mir, src.def_id);
    }
}

pub fn add_moves_for_packed_drops<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    mir: &mut Mir<'tcx>,
    def_id: DefId)
{
    let patch = add_moves_for_packed_drops_patch(tcx, mir, def_id);
    patch.apply(mir);
}

fn add_moves_for_packed_drops_patch<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    mir: &Mir<'tcx>,
    def_id: DefId)
    -> MirPatch<'tcx>
{
    let mut patch = MirPatch::new(mir);
    let param_env = tcx.param_env(def_id);

    for (bb, data) in mir.basic_blocks().iter_enumerated() {
        let loc = Location { block: bb, statement_index: data.statements.len() };
        let terminator = data.terminator();

        match terminator.kind {
            TerminatorKind::Drop { ref location, .. }
                if util::is_disaligned(tcx, mir, param_env, location) =>
            {
                add_move_for_packed_drop(tcx, mir, &mut patch, terminator,
                                         loc, data.is_cleanup);
            }
            TerminatorKind::DropAndReplace { .. } => {
                span_bug!(terminator.source_info.span,
                          "replace in AddMovesForPackedDrops");
            }
            _ => {}
        }
    }

    patch
}

fn add_move_for_packed_drop<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    mir: &Mir<'tcx>,
    patch: &mut MirPatch<'tcx>,
    terminator: &Terminator<'tcx>,
    loc: Location,
    is_cleanup: bool)
{
    debug!("add_move_for_packed_drop({:?} @ {:?})", terminator, loc);
    let (location, target, unwind) = match terminator.kind {
        TerminatorKind::Drop { ref location, target, unwind } =>
            (location, target, unwind),
        _ => unreachable!()
    };

    let source_info = terminator.source_info;
    let ty = location.ty(mir, tcx).to_ty(tcx);
    let temp = patch.new_temp(ty, terminator.source_info.span);

    let storage_dead_block = patch.new_block(BasicBlockData {
        statements: vec![Statement {
            source_info, kind: StatementKind::StorageDead(temp)
        }],
        terminator: Some(Terminator {
            source_info, kind: TerminatorKind::Goto { target }
        }),
        is_cleanup
    });

    patch.add_statement(
        loc, StatementKind::StorageLive(temp));
    patch.add_assign(loc, Place::Local(temp),
                     Rvalue::Use(Operand::Move(location.clone())));
    patch.patch_terminator(loc.block, TerminatorKind::Drop {
        location: Place::Local(temp),
        target: storage_dead_block,
        unwind
    });
}
