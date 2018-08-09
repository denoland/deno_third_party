// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*!
 * Methods for the various MIR types. These are intended for use after
 * building is complete.
 */

use mir::*;
use ty::subst::{Subst, Substs};
use ty::{self, AdtDef, Ty, TyCtxt};
use hir;
use ty::util::IntTypeExt;

#[derive(Copy, Clone, Debug)]
pub enum PlaceTy<'tcx> {
    /// Normal type.
    Ty { ty: Ty<'tcx> },

    /// Downcast to a particular variant of an enum.
    Downcast { adt_def: &'tcx AdtDef,
               substs: &'tcx Substs<'tcx>,
               variant_index: usize },
}

impl<'a, 'gcx, 'tcx> PlaceTy<'tcx> {
    pub fn from_ty(ty: Ty<'tcx>) -> PlaceTy<'tcx> {
        PlaceTy::Ty { ty: ty }
    }

    pub fn to_ty(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx> {
        match *self {
            PlaceTy::Ty { ty } =>
                ty,
            PlaceTy::Downcast { adt_def, substs, variant_index: _ } =>
                tcx.mk_adt(adt_def, substs),
        }
    }

    pub fn projection_ty(self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                         elem: &PlaceElem<'tcx>)
                         -> PlaceTy<'tcx>
    {
        match *elem {
            ProjectionElem::Deref => {
                let ty = self.to_ty(tcx)
                             .builtin_deref(true)
                             .unwrap_or_else(|| {
                                 bug!("deref projection of non-dereferencable ty {:?}", self)
                             })
                             .ty;
                PlaceTy::Ty {
                    ty,
                }
            }
            ProjectionElem::Index(_) | ProjectionElem::ConstantIndex { .. } =>
                PlaceTy::Ty {
                    ty: self.to_ty(tcx).builtin_index().unwrap()
                },
            ProjectionElem::Subslice { from, to } => {
                let ty = self.to_ty(tcx);
                PlaceTy::Ty {
                    ty: match ty.sty {
                        ty::TyArray(inner, size) => {
                            let size = size.unwrap_usize(tcx);
                            let len = size - (from as u64) - (to as u64);
                            tcx.mk_array(inner, len)
                        }
                        ty::TySlice(..) => ty,
                        _ => {
                            bug!("cannot subslice non-array type: `{:?}`", self)
                        }
                    }
                }
            }
            ProjectionElem::Downcast(adt_def1, index) =>
                match self.to_ty(tcx).sty {
                    ty::TyAdt(adt_def, substs) => {
                        assert!(adt_def.is_enum());
                        assert!(index < adt_def.variants.len());
                        assert_eq!(adt_def, adt_def1);
                        PlaceTy::Downcast { adt_def,
                                             substs,
                                             variant_index: index }
                    }
                    _ => {
                        bug!("cannot downcast non-ADT type: `{:?}`", self)
                    }
                },
            ProjectionElem::Field(_, fty) => PlaceTy::Ty { ty: fty }
        }
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for PlaceTy<'tcx> {
        (PlaceTy::Ty) { ty },
        (PlaceTy::Downcast) { adt_def, substs, variant_index },
    }
}

impl<'tcx> Place<'tcx> {
    pub fn ty<'a, 'gcx, D>(&self, local_decls: &D, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> PlaceTy<'tcx>
        where D: HasLocalDecls<'tcx>
    {
        match *self {
            Place::Local(index) =>
                PlaceTy::Ty { ty: local_decls.local_decls()[index].ty },
            Place::Static(ref data) =>
                PlaceTy::Ty { ty: data.ty },
            Place::Projection(ref proj) =>
                proj.base.ty(local_decls, tcx).projection_ty(tcx, &proj.elem),
        }
    }
}

pub enum RvalueInitializationState {
    Shallow,
    Deep
}

impl<'tcx> Rvalue<'tcx> {
    pub fn ty<'a, 'gcx, D>(&self, local_decls: &D, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx>
        where D: HasLocalDecls<'tcx>
    {
        match *self {
            Rvalue::Use(ref operand) => operand.ty(local_decls, tcx),
            Rvalue::Repeat(ref operand, count) => {
                tcx.mk_array(operand.ty(local_decls, tcx), count)
            }
            Rvalue::Ref(reg, bk, ref place) => {
                let place_ty = place.ty(local_decls, tcx).to_ty(tcx);
                tcx.mk_ref(reg,
                    ty::TypeAndMut {
                        ty: place_ty,
                        mutbl: bk.to_mutbl_lossy()
                    }
                )
            }
            Rvalue::Len(..) => tcx.types.usize,
            Rvalue::Cast(.., ty) => ty,
            Rvalue::BinaryOp(op, ref lhs, ref rhs) => {
                let lhs_ty = lhs.ty(local_decls, tcx);
                let rhs_ty = rhs.ty(local_decls, tcx);
                op.ty(tcx, lhs_ty, rhs_ty)
            }
            Rvalue::CheckedBinaryOp(op, ref lhs, ref rhs) => {
                let lhs_ty = lhs.ty(local_decls, tcx);
                let rhs_ty = rhs.ty(local_decls, tcx);
                let ty = op.ty(tcx, lhs_ty, rhs_ty);
                tcx.intern_tup(&[ty, tcx.types.bool])
            }
            Rvalue::UnaryOp(UnOp::Not, ref operand) |
            Rvalue::UnaryOp(UnOp::Neg, ref operand) => {
                operand.ty(local_decls, tcx)
            }
            Rvalue::Discriminant(ref place) => {
                let ty = place.ty(local_decls, tcx).to_ty(tcx);
                if let ty::TyAdt(adt_def, _) = ty.sty {
                    adt_def.repr.discr_type().to_ty(tcx)
                } else {
                    // This can only be `0`, for now, so `u8` will suffice.
                    tcx.types.u8
                }
            }
            Rvalue::NullaryOp(NullOp::Box, t) => tcx.mk_box(t),
            Rvalue::NullaryOp(NullOp::SizeOf, _) => tcx.types.usize,
            Rvalue::Aggregate(ref ak, ref ops) => {
                match **ak {
                    AggregateKind::Array(ty) => {
                        tcx.mk_array(ty, ops.len() as u64)
                    }
                    AggregateKind::Tuple => {
                        tcx.mk_tup(ops.iter().map(|op| op.ty(local_decls, tcx)))
                    }
                    AggregateKind::Adt(def, _, substs, _) => {
                        tcx.type_of(def.did).subst(tcx, substs)
                    }
                    AggregateKind::Closure(did, substs) => {
                        tcx.mk_closure(did, substs)
                    }
                    AggregateKind::Generator(did, substs, movability) => {
                        tcx.mk_generator(did, substs, movability)
                    }
                }
            }
        }
    }

    #[inline]
    /// Returns whether this rvalue is deeply initialized (most rvalues) or
    /// whether its only shallowly initialized (`Rvalue::Box`).
    pub fn initialization_state(&self) -> RvalueInitializationState {
        match *self {
            Rvalue::NullaryOp(NullOp::Box, _) => RvalueInitializationState::Shallow,
            _ => RvalueInitializationState::Deep
        }
    }
}

impl<'tcx> Operand<'tcx> {
    pub fn ty<'a, 'gcx, D>(&self, local_decls: &D, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx>
        where D: HasLocalDecls<'tcx>
    {
        match self {
            &Operand::Copy(ref l) |
            &Operand::Move(ref l) => l.ty(local_decls, tcx).to_ty(tcx),
            &Operand::Constant(ref c) => c.ty,
        }
    }
}

impl<'tcx> BinOp {
      pub fn ty<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                    lhs_ty: Ty<'tcx>,
                    rhs_ty: Ty<'tcx>)
                    -> Ty<'tcx> {
        // FIXME: handle SIMD correctly
        match self {
            &BinOp::Add | &BinOp::Sub | &BinOp::Mul | &BinOp::Div | &BinOp::Rem |
            &BinOp::BitXor | &BinOp::BitAnd | &BinOp::BitOr => {
                // these should be integers or floats of the same size.
                assert_eq!(lhs_ty, rhs_ty);
                lhs_ty
            }
            &BinOp::Shl | &BinOp::Shr | &BinOp::Offset => {
                lhs_ty // lhs_ty can be != rhs_ty
            }
            &BinOp::Eq | &BinOp::Lt | &BinOp::Le |
            &BinOp::Ne | &BinOp::Ge | &BinOp::Gt => {
                tcx.types.bool
            }
        }
    }
}

impl BorrowKind {
    pub fn to_mutbl_lossy(self) -> hir::Mutability {
        match self {
            BorrowKind::Mut { .. } => hir::MutMutable,
            BorrowKind::Shared => hir::MutImmutable,

            // We have no type corresponding to a unique imm borrow, so
            // use `&mut`. It gives all the capabilities of an `&uniq`
            // and hence is a safe "over approximation".
            BorrowKind::Unique => hir::MutMutable,
        }
    }
}

impl BinOp {
    pub fn to_hir_binop(self) -> hir::BinOp_ {
        match self {
            BinOp::Add => hir::BinOp_::BiAdd,
            BinOp::Sub => hir::BinOp_::BiSub,
            BinOp::Mul => hir::BinOp_::BiMul,
            BinOp::Div => hir::BinOp_::BiDiv,
            BinOp::Rem => hir::BinOp_::BiRem,
            BinOp::BitXor => hir::BinOp_::BiBitXor,
            BinOp::BitAnd => hir::BinOp_::BiBitAnd,
            BinOp::BitOr => hir::BinOp_::BiBitOr,
            BinOp::Shl => hir::BinOp_::BiShl,
            BinOp::Shr => hir::BinOp_::BiShr,
            BinOp::Eq => hir::BinOp_::BiEq,
            BinOp::Ne => hir::BinOp_::BiNe,
            BinOp::Lt => hir::BinOp_::BiLt,
            BinOp::Gt => hir::BinOp_::BiGt,
            BinOp::Le => hir::BinOp_::BiLe,
            BinOp::Ge => hir::BinOp_::BiGe,
            BinOp::Offset => unreachable!()
        }
    }
}
