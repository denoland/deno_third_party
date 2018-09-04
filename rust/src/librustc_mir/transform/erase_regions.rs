// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This pass erases all early-bound regions from the types occurring in the MIR.
//! We want to do this once just before codegen, so codegen does not have to take
//! care erasing regions all over the place.
//! NOTE:  We do NOT erase regions of statements that are relevant for
//! "types-as-contracts"-validation, namely, AcquireValid, ReleaseValid, and EndRegion.

use rustc::ty::subst::Substs;
use rustc::ty::{self, Ty, TyCtxt};
use rustc::mir::*;
use rustc::mir::visit::{MutVisitor, TyContext};
use transform::{MirPass, MirSource};

struct EraseRegionsVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    in_validation_statement: bool,
}

impl<'a, 'tcx> EraseRegionsVisitor<'a, 'tcx> {
    pub fn new(tcx: TyCtxt<'a, 'tcx, 'tcx>) -> Self {
        EraseRegionsVisitor {
            tcx,
            in_validation_statement: false,
        }
    }
}

impl<'a, 'tcx> MutVisitor<'tcx> for EraseRegionsVisitor<'a, 'tcx> {
    fn visit_ty(&mut self, ty: &mut Ty<'tcx>, _: TyContext) {
        if !self.in_validation_statement {
            *ty = self.tcx.erase_regions(ty);
        }
        self.super_ty(ty);
    }

    fn visit_region(&mut self, region: &mut ty::Region<'tcx>, _: Location) {
        *region = self.tcx.types.re_erased;
    }

    fn visit_const(&mut self, constant: &mut &'tcx ty::Const<'tcx>, _: Location) {
        *constant = self.tcx.erase_regions(constant);
    }

    fn visit_substs(&mut self, substs: &mut &'tcx Substs<'tcx>, _: Location) {
        *substs = self.tcx.erase_regions(substs);
    }

    fn visit_statement(&mut self,
                       block: BasicBlock,
                       statement: &mut Statement<'tcx>,
                       location: Location) {
        // Do NOT delete EndRegion if validation statements are emitted.
        // Validation needs EndRegion.
        if self.tcx.sess.opts.debugging_opts.mir_emit_validate == 0 {
            if let StatementKind::EndRegion(_) = statement.kind {
                statement.kind = StatementKind::Nop;
            }
        }

        self.in_validation_statement = match statement.kind {
            StatementKind::Validate(..) => true,
            _ => false,
        };
        self.super_statement(block, statement, location);
        self.in_validation_statement = false;
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
