//! This pass erases all early-bound regions from the types occurring in the MIR.
//! We want to do this once just before codegen, so codegen does not have to take
//! care erasing regions all over the place.
//! NOTE:  We do NOT erase regions of statements that are relevant for
//! "types-as-contracts"-validation, namely, AcquireValid, ReleaseValid

use rustc::ty::subst::Substs;
use rustc::ty::{self, Ty, TyCtxt};
use rustc::mir::*;
use rustc::mir::visit::{MutVisitor, TyContext};
use transform::{MirPass, MirSource};

struct EraseRegionsVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
}

impl<'a, 'tcx> EraseRegionsVisitor<'a, 'tcx> {
    pub fn new(tcx: TyCtxt<'a, 'tcx, 'tcx>) -> Self {
        EraseRegionsVisitor {
            tcx,
        }
    }
}

impl<'a, 'tcx> MutVisitor<'tcx> for EraseRegionsVisitor<'a, 'tcx> {
    fn visit_ty(&mut self, ty: &mut Ty<'tcx>, _: TyContext) {
        *ty = self.tcx.erase_regions(ty);
        self.super_ty(ty);
    }

    fn visit_region(&mut self, region: &mut ty::Region<'tcx>, _: Location) {
        *region = self.tcx.types.re_erased;
    }

    fn visit_const(&mut self, constant: &mut &'tcx ty::LazyConst<'tcx>, _: Location) {
        *constant = self.tcx.erase_regions(constant);
    }

    fn visit_substs(&mut self, substs: &mut &'tcx Substs<'tcx>, _: Location) {
        *substs = self.tcx.erase_regions(substs);
    }

    fn visit_statement(&mut self,
                       block: BasicBlock,
                       statement: &mut Statement<'tcx>,
                       location: Location) {
        self.super_statement(block, statement, location);
    }
}

pub struct EraseRegions;

impl MirPass for EraseRegions {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          _: MirSource,
                          mir: &mut Mir<'tcx>) {
        EraseRegionsVisitor::new(tcx).visit_mir(mir);
    }
}
