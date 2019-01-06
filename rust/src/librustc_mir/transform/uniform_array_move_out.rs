// This pass converts move out from array by Subslice and
// ConstIndex{.., from_end: true} to ConstIndex move out(s) from begin
// of array. It allows detect error by mir borrowck and elaborate
// drops for array without additional work.
//
// Example:
//
// let a = [ box 1,box 2, box 3];
// if b {
//  let [_a.., _] = a;
// } else {
//  let [.., _b] = a;
// }
//
//  mir statement _10 = move _2[:-1]; replaced by:
//  StorageLive(_12);
//  _12 = move _2[0 of 3];
//  StorageLive(_13);
//  _13 = move _2[1 of 3];
//  _10 = [move _12, move _13]
//  StorageDead(_12);
//  StorageDead(_13);
//
//  and mir statement _11 = move _2[-1 of 1]; replaced by:
//  _11 = move _2[2 of 3];
//
// FIXME: integrate this transformation to the mir build

use rustc::ty;
use rustc::ty::TyCtxt;
use rustc::mir::*;
use rustc::mir::visit::{Visitor, PlaceContext, NonUseContext};
use transform::{MirPass, MirSource};
use util::patch::MirPatch;
use rustc_data_structures::indexed_vec::{IndexVec};

pub struct UniformArrayMoveOut;

impl MirPass for UniformArrayMoveOut {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          _src: MirSource,
                          mir: &mut Mir<'tcx>) {
        let mut patch = MirPatch::new(mir);
        {
            let mut visitor = UniformArrayMoveOutVisitor{mir, patch: &mut patch, tcx};
            visitor.visit_mir(mir);
        }
        patch.apply(mir);
    }
}

struct UniformArrayMoveOutVisitor<'a, 'tcx: 'a> {
    mir: &'a Mir<'tcx>,
    patch: &'a mut MirPatch<'tcx>,
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
}

impl<'a, 'tcx> Visitor<'tcx> for UniformArrayMoveOutVisitor<'a, 'tcx> {
    fn visit_assign(&mut self,
                    block: BasicBlock,
                    dst_place: &Place<'tcx>,
                    rvalue: &Rvalue<'tcx>,
                    location: Location) {
        if let Rvalue::Use(Operand::Move(ref src_place)) = rvalue {
            if let Place::Projection(ref proj) = *src_place {
                if let ProjectionElem::ConstantIndex{offset: _,
                                                     min_length: _,
                                                     from_end: false} = proj.elem {
                    // no need to transformation
                } else {
                    let place_ty = proj.base.ty(self.mir, self.tcx).to_ty(self.tcx);
                    if let ty::Array(item_ty, const_size) = place_ty.sty {
                        if let Some(size) = const_size.assert_usize(self.tcx) {
                            assert!(size <= u32::max_value() as u64,
                                    "uniform array move out doesn't supported
                                     for array bigger then u32");
                            self.uniform(location, dst_place, proj, item_ty, size as u32);
                        }
                    }

                }
            }
        }
        self.super_assign(block, dst_place, rvalue, location)
    }
}

impl<'a, 'tcx> UniformArrayMoveOutVisitor<'a, 'tcx> {
    fn uniform(&mut self,
               location: Location,
               dst_place: &Place<'tcx>,
               proj: &PlaceProjection<'tcx>,
               item_ty: &'tcx ty::TyS<'tcx>,
               size: u32) {
        match proj.elem {
            // uniforms statements like_10 = move _2[:-1];
            ProjectionElem::Subslice{from, to} => {
                self.patch.make_nop(location);
                let temps : Vec<_> = (from..(size-to)).map(|i| {
                    let temp = self.patch.new_temp(item_ty, self.mir.source_info(location).span);
                    self.patch.add_statement(location, StatementKind::StorageLive(temp));
                    self.patch.add_assign(location,
                                          Place::Local(temp),
                                          Rvalue::Use(
                                              Operand::Move(
                                                  Place::Projection(box PlaceProjection{
                                                      base: proj.base.clone(),
                                                      elem: ProjectionElem::ConstantIndex{
                                                          offset: i,
                                                          min_length: size,
                                                          from_end: false}
                                                  }))));
                    temp
                }).collect();
                self.patch.add_assign(location,
                                      dst_place.clone(),
                                      Rvalue::Aggregate(box AggregateKind::Array(item_ty),
                                      temps.iter().map(
                                          |x| Operand::Move(Place::Local(*x))).collect()
                                      ));
                for temp in temps {
                    self.patch.add_statement(location, StatementKind::StorageDead(temp));
                }
            }
            // uniforms statements like _11 = move _2[-1 of 1];
            ProjectionElem::ConstantIndex{offset, min_length: _, from_end: true} => {
                self.patch.make_nop(location);
                self.patch.add_assign(location,
                                      dst_place.clone(),
                                      Rvalue::Use(
                                          Operand::Move(
                                              Place::Projection(box PlaceProjection{
                                                  base: proj.base.clone(),
                                                  elem: ProjectionElem::ConstantIndex{
                                                      offset: size - offset,
                                                      min_length: size,
                                                      from_end: false }}))));
            }
            _ => {}
        }
    }
}

// Restore Subslice move out after analysis
// Example:
//
//  next statements:
//   StorageLive(_12);
//   _12 = move _2[0 of 3];
//   StorageLive(_13);
//   _13 = move _2[1 of 3];
//   _10 = [move _12, move _13]
//   StorageDead(_12);
//   StorageDead(_13);
//
// replaced by _10 = move _2[:-1];

pub struct RestoreSubsliceArrayMoveOut;

impl MirPass for RestoreSubsliceArrayMoveOut {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          _src: MirSource,
                          mir: &mut Mir<'tcx>) {
        let mut patch = MirPatch::new(mir);
        {
            let mut visitor = RestoreDataCollector {
                locals_use: IndexVec::from_elem(LocalUse::new(), &mir.local_decls),
                candidates: vec![],
            };
            visitor.visit_mir(mir);

            for candidate in &visitor.candidates {
                let statement = &mir[candidate.block].statements[candidate.statement_index];
                if let StatementKind::Assign(ref dst_place, ref rval) = statement.kind {
                    if let Rvalue::Aggregate(box AggregateKind::Array(_), ref items) = **rval {
                        let items : Vec<_> = items.iter().map(|item| {
                            if let Operand::Move(Place::Local(local)) = item {
                                let local_use = &visitor.locals_use[*local];
                                let opt_index_and_place = Self::try_get_item_source(local_use, mir);
                                // each local should be used twice:
                                //  in assign and in aggregate statements
                                if local_use.use_count == 2 && opt_index_and_place.is_some() {
                                    let (index, src_place) = opt_index_and_place.unwrap();
                                    return Some((local_use, index, src_place));
                                }
                            }
                            None
                        }).collect();

                        let opt_src_place = items.first().and_then(|x| *x).map(|x| x.2);
                        let opt_size = opt_src_place.and_then(|src_place| {
                            let src_ty = src_place.ty(mir, tcx).to_ty(tcx);
                            if let ty::Array(_, ref size_o) = src_ty.sty {
                                size_o.assert_usize(tcx)
                            } else {
                                None
                            }
                        });
                        Self::check_and_patch(*candidate, &items, opt_size, &mut patch, dst_place);
                    }
                }
            }
        }
        patch.apply(mir);
    }
}

impl RestoreSubsliceArrayMoveOut {
    // Checks that source has size, all locals are inited from same source place and
    // indices is an integer interval. If all checks pass do the replacent.
    // items are Vec<Option<LocalUse, index in source array, source place for init local>>
    fn check_and_patch<'tcx>(candidate: Location,
                             items: &[Option<(&LocalUse, u32, &Place<'tcx>)>],
                             opt_size: Option<u64>,
                             patch: &mut MirPatch<'tcx>,
                             dst_place: &Place<'tcx>) {
        let opt_src_place = items.first().and_then(|x| *x).map(|x| x.2);

        if opt_size.is_some() && items.iter().all(
            |l| l.is_some() && l.unwrap().2 == opt_src_place.unwrap()) {

            let indices: Vec<_> = items.iter().map(|x| x.unwrap().1).collect();
            for i in 1..indices.len() {
                if indices[i - 1] + 1 != indices[i] {
                    return;
                }
            }

            let min = *indices.first().unwrap();
            let max = *indices.last().unwrap();

            for item in items {
                let locals_use = item.unwrap().0;
                patch.make_nop(locals_use.alive.unwrap());
                patch.make_nop(locals_use.dead.unwrap());
                patch.make_nop(locals_use.first_use.unwrap());
            }
            patch.make_nop(candidate);
            let size = opt_size.unwrap() as u32;
            patch.add_assign(candidate,
                             dst_place.clone(),
                             Rvalue::Use(
                                 Operand::Move(
                                     Place::Projection(box PlaceProjection{
                                         base: opt_src_place.unwrap().clone(),
                                         elem: ProjectionElem::Subslice{
                                             from: min, to: size - max - 1}}))));
        }
    }

    fn try_get_item_source<'a, 'tcx>(local_use: &LocalUse,
                                     mir: &'a Mir<'tcx>) -> Option<(u32, &'a Place<'tcx>)> {
        if let Some(location) = local_use.first_use {
            let block = &mir[location.block];
            if block.statements.len() > location.statement_index {
                let statement = &block.statements[location.statement_index];
                if let StatementKind::Assign(
                    Place::Local(_),
                    box Rvalue::Use(Operand::Move(Place::Projection(box PlaceProjection{
                        ref base, elem: ProjectionElem::ConstantIndex{
                            offset, min_length: _, from_end: false}})))) = statement.kind {
                    return Some((offset, base))
                }
            }
        }
        None
    }
}

#[derive(Copy, Clone, Debug)]
struct LocalUse {
    alive: Option<Location>,
    dead: Option<Location>,
    use_count: u32,
    first_use: Option<Location>,
}

impl LocalUse {
    pub fn new() -> Self {
        LocalUse{alive: None, dead: None, use_count: 0, first_use: None}
    }
}

struct RestoreDataCollector {
    locals_use: IndexVec<Local, LocalUse>,
    candidates: Vec<Location>,
}

impl<'tcx> Visitor<'tcx> for RestoreDataCollector {
    fn visit_assign(&mut self,
                    block: BasicBlock,
                    place: &Place<'tcx>,
                    rvalue: &Rvalue<'tcx>,
                    location: Location) {
        if let Rvalue::Aggregate(box AggregateKind::Array(_), _) = *rvalue {
            self.candidates.push(location);
        }
        self.super_assign(block, place, rvalue, location)
    }

    fn visit_local(&mut self,
                   local: &Local,
                   context: PlaceContext<'tcx>,
                   location: Location) {
        let local_use = &mut self.locals_use[*local];
        match context {
            PlaceContext::NonUse(NonUseContext::StorageLive) => local_use.alive = Some(location),
            PlaceContext::NonUse(NonUseContext::StorageDead) => local_use.dead = Some(location),
            _ => {
                local_use.use_count += 1;
                if local_use.first_use.is_none() {
                    local_use.first_use = Some(location);
                }
            }
        }
    }
}
