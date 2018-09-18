// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module contains implements of the `Lift` and `TypeFoldable`
//! traits for various types in the Rust compiler. Most are written by
//! hand, though we've recently added some macros (e.g.,
//! `BraceStructLiftImpl!`) to help with the tedium.

use middle::const_val::{self, ConstVal, ConstEvalErr};
use ty::{self, Lift, Ty, TyCtxt};
use ty::fold::{TypeFoldable, TypeFolder, TypeVisitor};
use rustc_data_structures::accumulate_vec::AccumulateVec;
use rustc_data_structures::indexed_vec::{IndexVec, Idx};
use rustc_data_structures::sync::Lrc;
use mir::interpret;

use std::rc::Rc;

///////////////////////////////////////////////////////////////////////////
// Atomic structs
//
// For things that don't carry any arena-allocated data (and are
// copy...), just add them to this list.

CloneTypeFoldableAndLiftImpls! {
    (),
    bool,
    usize,
    u64,
    ::middle::region::Scope,
    ::syntax::ast::FloatTy,
    ::syntax::ast::NodeId,
    ::syntax_pos::symbol::Symbol,
    ::hir::def::Def,
    ::hir::def_id::DefId,
    ::hir::InlineAsm,
    ::hir::MatchSource,
    ::hir::Mutability,
    ::hir::Unsafety,
    ::rustc_target::spec::abi::Abi,
    ::mir::Local,
    ::mir::Promoted,
    ::traits::Reveal,
    ::ty::adjustment::AutoBorrowMutability,
    ::ty::AdtKind,
    // Including `BoundRegion` is a *bit* dubious, but direct
    // references to bound region appear in `ty::Error`, and aren't
    // really meant to be folded. In general, we can only fold a fully
    // general `Region`.
    ::ty::BoundRegion,
    ::ty::ClosureKind,
    ::ty::IntVarValue,
    ::syntax_pos::Span,
}

///////////////////////////////////////////////////////////////////////////
// Lift implementations

impl<'tcx, A: Lift<'tcx>, B: Lift<'tcx>> Lift<'tcx> for (A, B) {
    type Lifted = (A::Lifted, B::Lifted);
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.0).and_then(|a| tcx.lift(&self.1).map(|b| (a, b)))
    }
}

impl<'tcx, A: Lift<'tcx>, B: Lift<'tcx>, C: Lift<'tcx>> Lift<'tcx> for (A, B, C) {
    type Lifted = (A::Lifted, B::Lifted, C::Lifted);
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.0).and_then(|a| {
            tcx.lift(&self.1).and_then(|b| tcx.lift(&self.2).map(|c| (a, b, c)))
        })
    }
}

impl<'tcx, T: Lift<'tcx>> Lift<'tcx> for Option<T> {
    type Lifted = Option<T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            Some(ref x) => tcx.lift(x).map(Some),
            None => Some(None)
        }
    }
}

impl<'tcx, T: Lift<'tcx>, E: Lift<'tcx>> Lift<'tcx> for Result<T, E> {
    type Lifted = Result<T::Lifted, E::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            Ok(ref x) => tcx.lift(x).map(Ok),
            Err(ref e) => tcx.lift(e).map(Err)
        }
    }
}

impl<'tcx, T: Lift<'tcx>> Lift<'tcx> for Box<T> {
    type Lifted = Box<T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&**self).map(Box::new)
    }
}

impl<'tcx, T: Lift<'tcx>> Lift<'tcx> for [T] {
    type Lifted = Vec<T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        // type annotation needed to inform `projection_must_outlive`
        let mut result : Vec<<T as Lift<'tcx>>::Lifted>
            = Vec::with_capacity(self.len());
        for x in self {
            if let Some(value) = tcx.lift(x) {
                result.push(value);
            } else {
                return None;
            }
        }
        Some(result)
    }
}

impl<'tcx, T: Lift<'tcx>> Lift<'tcx> for Vec<T> {
    type Lifted = Vec<T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self[..])
    }
}

impl<'tcx, I: Idx, T: Lift<'tcx>> Lift<'tcx> for IndexVec<I, T> {
    type Lifted = IndexVec<I, T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        self.iter()
            .map(|e| tcx.lift(e))
            .collect()
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::TraitRef<'a> {
    type Lifted = ty::TraitRef<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.substs).map(|substs| ty::TraitRef {
            def_id: self.def_id,
            substs,
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::ExistentialTraitRef<'a> {
    type Lifted = ty::ExistentialTraitRef<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.substs).map(|substs| ty::ExistentialTraitRef {
            def_id: self.def_id,
            substs,
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::TraitPredicate<'a> {
    type Lifted = ty::TraitPredicate<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>)
                             -> Option<ty::TraitPredicate<'tcx>> {
        tcx.lift(&self.trait_ref).map(|trait_ref| ty::TraitPredicate {
            trait_ref,
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::SubtypePredicate<'a> {
    type Lifted = ty::SubtypePredicate<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>)
                             -> Option<ty::SubtypePredicate<'tcx>> {
        tcx.lift(&(self.a, self.b)).map(|(a, b)| ty::SubtypePredicate {
            a_is_expected: self.a_is_expected,
            a,
            b,
        })
    }
}

impl<'tcx, A: Copy+Lift<'tcx>, B: Copy+Lift<'tcx>> Lift<'tcx> for ty::OutlivesPredicate<A, B> {
    type Lifted = ty::OutlivesPredicate<A::Lifted, B::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&(self.0, self.1)).map(|(a, b)| ty::OutlivesPredicate(a, b))
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::ProjectionTy<'a> {
    type Lifted = ty::ProjectionTy<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>)
                             -> Option<ty::ProjectionTy<'tcx>> {
        tcx.lift(&self.substs).map(|substs| {
            ty::ProjectionTy {
                item_def_id: self.item_def_id,
                substs,
            }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::ProjectionPredicate<'a> {
    type Lifted = ty::ProjectionPredicate<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>)
                             -> Option<ty::ProjectionPredicate<'tcx>> {
        tcx.lift(&(self.projection_ty, self.ty)).map(|(projection_ty, ty)| {
            ty::ProjectionPredicate {
                projection_ty,
                ty,
            }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::ExistentialProjection<'a> {
    type Lifted = ty::ExistentialProjection<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.substs).map(|substs| {
            ty::ExistentialProjection {
                substs,
                ty: tcx.lift(&self.ty).expect("type must lift when substs do"),
                item_def_id: self.item_def_id,
            }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::Predicate<'a> {
    type Lifted = ty::Predicate<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            ty::Predicate::Trait(ref binder) => {
                tcx.lift(binder).map(ty::Predicate::Trait)
            }
            ty::Predicate::Subtype(ref binder) => {
                tcx.lift(binder).map(ty::Predicate::Subtype)
            }
            ty::Predicate::RegionOutlives(ref binder) => {
                tcx.lift(binder).map(ty::Predicate::RegionOutlives)
            }
            ty::Predicate::TypeOutlives(ref binder) => {
                tcx.lift(binder).map(ty::Predicate::TypeOutlives)
            }
            ty::Predicate::Projection(ref binder) => {
                tcx.lift(binder).map(ty::Predicate::Projection)
            }
            ty::Predicate::WellFormed(ty) => {
                tcx.lift(&ty).map(ty::Predicate::WellFormed)
            }
            ty::Predicate::ClosureKind(closure_def_id, closure_substs, kind) => {
                tcx.lift(&closure_substs)
                   .map(|closure_substs| ty::Predicate::ClosureKind(closure_def_id,
                                                                    closure_substs,
                                                                    kind))
            }
            ty::Predicate::ObjectSafe(trait_def_id) => {
                Some(ty::Predicate::ObjectSafe(trait_def_id))
            }
            ty::Predicate::ConstEvaluatable(def_id, substs) => {
                tcx.lift(&substs).map(|substs| {
                    ty::Predicate::ConstEvaluatable(def_id, substs)
                })
            }
        }
    }
}

impl<'tcx, T: Lift<'tcx>> Lift<'tcx> for ty::Binder<T> {
    type Lifted = ty::Binder<T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(self.skip_binder()).map(ty::Binder::bind)
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::ParamEnv<'a> {
    type Lifted = ty::ParamEnv<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.caller_bounds).map(|caller_bounds| {
            ty::ParamEnv {
                reveal: self.reveal,
                caller_bounds,
            }
        })
    }
}

impl<'a, 'tcx, T: Lift<'tcx>> Lift<'tcx> for ty::ParamEnvAnd<'a, T> {
    type Lifted = ty::ParamEnvAnd<'tcx, T::Lifted>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.param_env).and_then(|param_env| {
            tcx.lift(&self.value).map(|value| {
                ty::ParamEnvAnd {
                    param_env,
                    value,
                }
            })
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::ClosureSubsts<'a> {
    type Lifted = ty::ClosureSubsts<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.substs).map(|substs| {
            ty::ClosureSubsts { substs }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::GeneratorSubsts<'a> {
    type Lifted = ty::GeneratorSubsts<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.substs).map(|substs| {
            ty::GeneratorSubsts { substs }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::adjustment::Adjustment<'a> {
    type Lifted = ty::adjustment::Adjustment<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.kind).and_then(|kind| {
            tcx.lift(&self.target).map(|target| {
                ty::adjustment::Adjustment { kind, target }
            })
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::adjustment::Adjust<'a> {
    type Lifted = ty::adjustment::Adjust<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            ty::adjustment::Adjust::NeverToAny =>
                Some(ty::adjustment::Adjust::NeverToAny),
            ty::adjustment::Adjust::ReifyFnPointer =>
                Some(ty::adjustment::Adjust::ReifyFnPointer),
            ty::adjustment::Adjust::UnsafeFnPointer =>
                Some(ty::adjustment::Adjust::UnsafeFnPointer),
            ty::adjustment::Adjust::ClosureFnPointer =>
                Some(ty::adjustment::Adjust::ClosureFnPointer),
            ty::adjustment::Adjust::MutToConstPointer =>
                Some(ty::adjustment::Adjust::MutToConstPointer),
            ty::adjustment::Adjust::Unsize =>
                Some(ty::adjustment::Adjust::Unsize),
            ty::adjustment::Adjust::Deref(ref overloaded) => {
                tcx.lift(overloaded).map(ty::adjustment::Adjust::Deref)
            }
            ty::adjustment::Adjust::Borrow(ref autoref) => {
                tcx.lift(autoref).map(ty::adjustment::Adjust::Borrow)
            }
        }
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::adjustment::OverloadedDeref<'a> {
    type Lifted = ty::adjustment::OverloadedDeref<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.region).map(|region| {
            ty::adjustment::OverloadedDeref {
                region,
                mutbl: self.mutbl,
            }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::adjustment::AutoBorrow<'a> {
    type Lifted = ty::adjustment::AutoBorrow<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            ty::adjustment::AutoBorrow::Ref(r, m) => {
                tcx.lift(&r).map(|r| ty::adjustment::AutoBorrow::Ref(r, m))
            }
            ty::adjustment::AutoBorrow::RawPtr(m) => {
                Some(ty::adjustment::AutoBorrow::RawPtr(m))
            }
        }
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::GenSig<'a> {
    type Lifted = ty::GenSig<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&(self.yield_ty, self.return_ty))
            .map(|(yield_ty, return_ty)| {
                ty::GenSig {
                    yield_ty,
                    return_ty,
                }
            })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::FnSig<'a> {
    type Lifted = ty::FnSig<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.inputs_and_output).map(|x| {
            ty::FnSig {
                inputs_and_output: x,
                variadic: self.variadic,
                unsafety: self.unsafety,
                abi: self.abi,
            }
        })
    }
}

impl<'tcx, T: Lift<'tcx>> Lift<'tcx> for ty::error::ExpectedFound<T> {
    type Lifted = ty::error::ExpectedFound<T::Lifted>;
    fn lift_to_tcx<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&self.expected).and_then(|expected| {
            tcx.lift(&self.found).map(|found| {
                ty::error::ExpectedFound {
                    expected,
                    found,
                }
            })
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::error::TypeError<'a> {
    type Lifted = ty::error::TypeError<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        use ty::error::TypeError::*;

        Some(match *self {
            Mismatch => Mismatch,
            UnsafetyMismatch(x) => UnsafetyMismatch(x),
            AbiMismatch(x) => AbiMismatch(x),
            Mutability => Mutability,
            TupleSize(x) => TupleSize(x),
            FixedArraySize(x) => FixedArraySize(x),
            ArgCount => ArgCount,
            RegionsDoesNotOutlive(a, b) => {
                return tcx.lift(&(a, b)).map(|(a, b)| RegionsDoesNotOutlive(a, b))
            }
            RegionsInsufficientlyPolymorphic(a, b) => {
                return tcx.lift(&b).map(|b| RegionsInsufficientlyPolymorphic(a, b))
            }
            RegionsOverlyPolymorphic(a, b) => {
                return tcx.lift(&b).map(|b| RegionsOverlyPolymorphic(a, b))
            }
            IntMismatch(x) => IntMismatch(x),
            FloatMismatch(x) => FloatMismatch(x),
            Traits(x) => Traits(x),
            VariadicMismatch(x) => VariadicMismatch(x),
            CyclicTy(t) => return tcx.lift(&t).map(|t| CyclicTy(t)),
            ProjectionMismatched(x) => ProjectionMismatched(x),
            ProjectionBoundsLength(x) => ProjectionBoundsLength(x),

            Sorts(ref x) => return tcx.lift(x).map(Sorts),
            OldStyleLUB(ref x) => return tcx.lift(x).map(OldStyleLUB),
            ExistentialMismatch(ref x) => return tcx.lift(x).map(ExistentialMismatch)
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ConstEvalErr<'a> {
    type Lifted = ConstEvalErr<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        tcx.lift(&*self.kind).map(|kind| {
            ConstEvalErr {
                span: self.span,
                kind: Lrc::new(kind),
            }
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for interpret::EvalError<'a> {
    type Lifted = interpret::EvalError<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        Some(interpret::EvalError {
            kind: tcx.lift(&self.kind)?,
        })
    }
}

impl<'a, 'tcx, O: Lift<'tcx>> Lift<'tcx> for interpret::EvalErrorKind<'a, O> {
    type Lifted = interpret::EvalErrorKind<'tcx, <O as Lift<'tcx>>::Lifted>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        use ::mir::interpret::EvalErrorKind::*;
        Some(match *self {
            MachineError(ref err) => MachineError(err.clone()),
            FunctionPointerTyMismatch(a, b) => FunctionPointerTyMismatch(
                tcx.lift(&a)?,
                tcx.lift(&b)?,
            ),
            NoMirFor(ref s) => NoMirFor(s.clone()),
            UnterminatedCString(ptr) => UnterminatedCString(ptr),
            DanglingPointerDeref => DanglingPointerDeref,
            DoubleFree => DoubleFree,
            InvalidMemoryAccess => InvalidMemoryAccess,
            InvalidFunctionPointer => InvalidFunctionPointer,
            InvalidBool => InvalidBool,
            InvalidDiscriminant => InvalidDiscriminant,
            PointerOutOfBounds {
                ptr,
                access,
                allocation_size,
            } => PointerOutOfBounds { ptr, access, allocation_size },
            InvalidNullPointerUsage => InvalidNullPointerUsage,
            ReadPointerAsBytes => ReadPointerAsBytes,
            ReadBytesAsPointer => ReadBytesAsPointer,
            InvalidPointerMath => InvalidPointerMath,
            ReadUndefBytes => ReadUndefBytes,
            DeadLocal => DeadLocal,
            InvalidBoolOp(bop) => InvalidBoolOp(bop),
            Unimplemented(ref s) => Unimplemented(s.clone()),
            DerefFunctionPointer => DerefFunctionPointer,
            ExecuteMemory => ExecuteMemory,
            BoundsCheck { ref len, ref index } => BoundsCheck {
                len: tcx.lift(len)?,
                index: tcx.lift(index)?,
            },
            Intrinsic(ref s) => Intrinsic(s.clone()),
            InvalidChar(c) => InvalidChar(c),
            StackFrameLimitReached => StackFrameLimitReached,
            OutOfTls => OutOfTls,
            TlsOutOfBounds => TlsOutOfBounds,
            AbiViolation(ref s) => AbiViolation(s.clone()),
            AlignmentCheckFailed {
                required,
                has,
            } => AlignmentCheckFailed { required, has },
            MemoryLockViolation {
                ptr,
                len,
                frame,
                access,
                ref lock,
            } => MemoryLockViolation { ptr, len, frame, access, lock: lock.clone() },
            MemoryAcquireConflict {
                ptr,
                len,
                kind,
                ref lock,
            } => MemoryAcquireConflict { ptr, len, kind, lock: lock.clone() },
            InvalidMemoryLockRelease {
                ptr,
                len,
                frame,
                ref lock,
            } => InvalidMemoryLockRelease { ptr, len, frame, lock: lock.clone() },
            DeallocatedLockedMemory {
                ptr,
                ref lock,
            } => DeallocatedLockedMemory { ptr, lock: lock.clone() },
            ValidationFailure(ref s) => ValidationFailure(s.clone()),
            CalledClosureAsFunction => CalledClosureAsFunction,
            VtableForArgumentlessMethod => VtableForArgumentlessMethod,
            ModifiedConstantMemory => ModifiedConstantMemory,
            AssumptionNotHeld => AssumptionNotHeld,
            InlineAsm => InlineAsm,
            TypeNotPrimitive(ty) => TypeNotPrimitive(tcx.lift(&ty)?),
            ReallocatedWrongMemoryKind(ref a, ref b) => {
                ReallocatedWrongMemoryKind(a.clone(), b.clone())
            },
            DeallocatedWrongMemoryKind(ref a, ref b) => {
                DeallocatedWrongMemoryKind(a.clone(), b.clone())
            },
            ReallocateNonBasePtr => ReallocateNonBasePtr,
            DeallocateNonBasePtr => DeallocateNonBasePtr,
            IncorrectAllocationInformation(a, b, c, d) => {
                IncorrectAllocationInformation(a, b, c, d)
            },
            Layout(lay) => Layout(tcx.lift(&lay)?),
            HeapAllocZeroBytes => HeapAllocZeroBytes,
            HeapAllocNonPowerOfTwoAlignment(n) => HeapAllocNonPowerOfTwoAlignment(n),
            Unreachable => Unreachable,
            Panic => Panic,
            ReadFromReturnPointer => ReadFromReturnPointer,
            PathNotFound(ref v) => PathNotFound(v.clone()),
            UnimplementedTraitSelection => UnimplementedTraitSelection,
            TypeckError => TypeckError,
            ReferencedConstant(ref err) => ReferencedConstant(tcx.lift(err)?),
            OverflowNeg => OverflowNeg,
            Overflow(op) => Overflow(op),
            DivisionByZero => DivisionByZero,
            RemainderByZero => RemainderByZero,
            GeneratorResumedAfterReturn => GeneratorResumedAfterReturn,
            GeneratorResumedAfterPanic => GeneratorResumedAfterPanic,
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for const_val::ErrKind<'a> {
    type Lifted = const_val::ErrKind<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        use middle::const_val::ErrKind::*;

        Some(match *self {
            CouldNotResolve => CouldNotResolve,
            TypeckError => TypeckError,
            CheckMatchError => CheckMatchError,
            Miri(ref e, ref frames) => return tcx.lift(e).map(|e| Miri(e, frames.clone())),
        })
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::layout::LayoutError<'a> {
    type Lifted = ty::layout::LayoutError<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            ty::layout::LayoutError::Unknown(ref ty) => {
                tcx.lift(ty).map(ty::layout::LayoutError::Unknown)
            }
            ty::layout::LayoutError::SizeOverflow(ref ty) => {
                tcx.lift(ty).map(ty::layout::LayoutError::SizeOverflow)
            }
        }
    }
}

impl<'a, 'tcx> Lift<'tcx> for ty::InstanceDef<'a> {
    type Lifted = ty::InstanceDef<'tcx>;
    fn lift_to_tcx<'b, 'gcx>(&self, tcx: TyCtxt<'b, 'gcx, 'tcx>) -> Option<Self::Lifted> {
        match *self {
            ty::InstanceDef::Item(def_id) =>
                Some(ty::InstanceDef::Item(def_id)),
            ty::InstanceDef::Intrinsic(def_id) =>
                Some(ty::InstanceDef::Intrinsic(def_id)),
            ty::InstanceDef::FnPtrShim(def_id, ref ty) =>
                Some(ty::InstanceDef::FnPtrShim(def_id, tcx.lift(ty)?)),
            ty::InstanceDef::Virtual(def_id, n) =>
                Some(ty::InstanceDef::Virtual(def_id, n)),
            ty::InstanceDef::ClosureOnceShim { call_once } =>
                Some(ty::InstanceDef::ClosureOnceShim { call_once }),
            ty::InstanceDef::DropGlue(def_id, ref ty) =>
                Some(ty::InstanceDef::DropGlue(def_id, tcx.lift(ty)?)),
            ty::InstanceDef::CloneShim(def_id, ref ty) =>
                Some(ty::InstanceDef::CloneShim(def_id, tcx.lift(ty)?)),
        }
    }
}

BraceStructLiftImpl! {
    impl<'a, 'tcx> Lift<'tcx> for ty::Instance<'a> {
        type Lifted = ty::Instance<'tcx>;
        def, substs
    }
}

BraceStructLiftImpl! {
    impl<'a, 'tcx> Lift<'tcx> for interpret::GlobalId<'a> {
        type Lifted = interpret::GlobalId<'tcx>;
        instance, promoted
    }
}

///////////////////////////////////////////////////////////////////////////
// TypeFoldable implementations.
//
// Ideally, each type should invoke `folder.fold_foo(self)` and
// nothing else. In some cases, though, we haven't gotten around to
// adding methods on the `folder` yet, and thus the folding is
// hard-coded here. This is less-flexible, because folders cannot
// override the behavior, but there are a lot of random types and one
// can easily refactor the folding into the TypeFolder trait as
// needed.

/// AdtDefs are basically the same as a DefId.
impl<'tcx> TypeFoldable<'tcx> for &'tcx ty::AdtDef {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, _folder: &mut F) -> Self {
        *self
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, _visitor: &mut V) -> bool {
        false
    }
}

impl<'tcx, T:TypeFoldable<'tcx>, U:TypeFoldable<'tcx>> TypeFoldable<'tcx> for (T, U) {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> (T, U) {
        (self.0.fold_with(folder), self.1.fold_with(folder))
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.0.visit_with(visitor) || self.1.visit_with(visitor)
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx, T> TypeFoldable<'tcx> for Option<T> {
        (Some)(a),
        (None),
    } where T: TypeFoldable<'tcx>
}

impl<'tcx, T: TypeFoldable<'tcx>> TypeFoldable<'tcx> for Rc<T> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        Rc::new((**self).fold_with(folder))
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        (**self).visit_with(visitor)
    }
}

impl<'tcx, T: TypeFoldable<'tcx>> TypeFoldable<'tcx> for Box<T> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        let content: T = (**self).fold_with(folder);
        box content
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        (**self).visit_with(visitor)
    }
}

impl<'tcx, T: TypeFoldable<'tcx>> TypeFoldable<'tcx> for Vec<T> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        self.iter().map(|t| t.fold_with(folder)).collect()
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.iter().any(|t| t.visit_with(visitor))
    }
}

impl<'tcx, T:TypeFoldable<'tcx>> TypeFoldable<'tcx> for ty::Binder<T> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        self.map_bound_ref(|ty| ty.fold_with(folder))
    }

    fn fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        folder.fold_binder(self)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.skip_binder().visit_with(visitor)
    }

    fn visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        visitor.visit_binder(self)
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ParamEnv<'tcx> { reveal, caller_bounds }
}

impl<'tcx> TypeFoldable<'tcx> for &'tcx ty::Slice<ty::ExistentialPredicate<'tcx>> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        let v = self.iter().map(|p| p.fold_with(folder)).collect::<AccumulateVec<[_; 8]>>();
        folder.tcx().intern_existential_predicates(&v)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.iter().any(|p| p.visit_with(visitor))
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ExistentialPredicate<'tcx> {
        (ty::ExistentialPredicate::Trait)(a),
        (ty::ExistentialPredicate::Projection)(a),
        (ty::ExistentialPredicate::AutoTrait)(a),
    }
}

impl<'tcx> TypeFoldable<'tcx> for &'tcx ty::Slice<Ty<'tcx>> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        let v = self.iter().map(|t| t.fold_with(folder)).collect::<AccumulateVec<[_; 8]>>();
        folder.tcx().intern_type_list(&v)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.iter().any(|t| t.visit_with(visitor))
    }
}

impl<'tcx> TypeFoldable<'tcx> for ty::instance::Instance<'tcx> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        use ty::InstanceDef::*;
        Self {
            substs: self.substs.fold_with(folder),
            def: match self.def {
                Item(did) => Item(did.fold_with(folder)),
                Intrinsic(did) => Intrinsic(did.fold_with(folder)),
                FnPtrShim(did, ty) => FnPtrShim(
                    did.fold_with(folder),
                    ty.fold_with(folder),
                ),
                Virtual(did, i) => Virtual(
                    did.fold_with(folder),
                    i,
                ),
                ClosureOnceShim { call_once } => ClosureOnceShim {
                    call_once: call_once.fold_with(folder),
                },
                DropGlue(did, ty) => DropGlue(
                    did.fold_with(folder),
                    ty.fold_with(folder),
                ),
                CloneShim(did, ty) => CloneShim(
                    did.fold_with(folder),
                    ty.fold_with(folder),
                ),
            },
        }
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        use ty::InstanceDef::*;
        self.substs.visit_with(visitor) ||
        match self.def {
            Item(did) => did.visit_with(visitor),
            Intrinsic(did) => did.visit_with(visitor),
            FnPtrShim(did, ty) => {
                did.visit_with(visitor) ||
                ty.visit_with(visitor)
            },
            Virtual(did, _) => did.visit_with(visitor),
            ClosureOnceShim { call_once } => call_once.visit_with(visitor),
            DropGlue(did, ty) => {
                did.visit_with(visitor) ||
                ty.visit_with(visitor)
            },
            CloneShim(did, ty) => {
                did.visit_with(visitor) ||
                ty.visit_with(visitor)
            },
        }
    }
}

impl<'tcx> TypeFoldable<'tcx> for interpret::GlobalId<'tcx> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        Self {
            instance: self.instance.fold_with(folder),
            promoted: self.promoted
        }
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.instance.visit_with(visitor)
    }
}

impl<'tcx> TypeFoldable<'tcx> for Ty<'tcx> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        let sty = match self.sty {
            ty::TyRawPtr(tm) => ty::TyRawPtr(tm.fold_with(folder)),
            ty::TyArray(typ, sz) => ty::TyArray(typ.fold_with(folder), sz.fold_with(folder)),
            ty::TySlice(typ) => ty::TySlice(typ.fold_with(folder)),
            ty::TyAdt(tid, substs) => ty::TyAdt(tid, substs.fold_with(folder)),
            ty::TyDynamic(ref trait_ty, ref region) =>
                ty::TyDynamic(trait_ty.fold_with(folder), region.fold_with(folder)),
            ty::TyTuple(ts) => ty::TyTuple(ts.fold_with(folder)),
            ty::TyFnDef(def_id, substs) => {
                ty::TyFnDef(def_id, substs.fold_with(folder))
            }
            ty::TyFnPtr(f) => ty::TyFnPtr(f.fold_with(folder)),
            ty::TyRef(ref r, ty, mutbl) => {
                ty::TyRef(r.fold_with(folder), ty.fold_with(folder), mutbl)
            }
            ty::TyGenerator(did, substs, movability) => {
                ty::TyGenerator(
                    did,
                    substs.fold_with(folder),
                    movability)
            }
            ty::TyGeneratorWitness(types) => ty::TyGeneratorWitness(types.fold_with(folder)),
            ty::TyClosure(did, substs) => ty::TyClosure(did, substs.fold_with(folder)),
            ty::TyProjection(ref data) => ty::TyProjection(data.fold_with(folder)),
            ty::TyAnon(did, substs) => ty::TyAnon(did, substs.fold_with(folder)),
            ty::TyBool | ty::TyChar | ty::TyStr | ty::TyInt(_) |
            ty::TyUint(_) | ty::TyFloat(_) | ty::TyError | ty::TyInfer(_) |
            ty::TyParam(..) | ty::TyNever | ty::TyForeign(..) => return self
        };

        if self.sty == sty {
            self
        } else {
            folder.tcx().mk_ty(sty)
        }
    }

    fn fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        folder.fold_ty(*self)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        match self.sty {
            ty::TyRawPtr(ref tm) => tm.visit_with(visitor),
            ty::TyArray(typ, sz) => typ.visit_with(visitor) || sz.visit_with(visitor),
            ty::TySlice(typ) => typ.visit_with(visitor),
            ty::TyAdt(_, substs) => substs.visit_with(visitor),
            ty::TyDynamic(ref trait_ty, ref reg) =>
                trait_ty.visit_with(visitor) || reg.visit_with(visitor),
            ty::TyTuple(ts) => ts.visit_with(visitor),
            ty::TyFnDef(_, substs) => substs.visit_with(visitor),
            ty::TyFnPtr(ref f) => f.visit_with(visitor),
            ty::TyRef(r, ty, _) => r.visit_with(visitor) || ty.visit_with(visitor),
            ty::TyGenerator(_did, ref substs, _) => {
                substs.visit_with(visitor)
            }
            ty::TyGeneratorWitness(ref types) => types.visit_with(visitor),
            ty::TyClosure(_did, ref substs) => substs.visit_with(visitor),
            ty::TyProjection(ref data) => data.visit_with(visitor),
            ty::TyAnon(_, ref substs) => substs.visit_with(visitor),
            ty::TyBool | ty::TyChar | ty::TyStr | ty::TyInt(_) |
            ty::TyUint(_) | ty::TyFloat(_) | ty::TyError | ty::TyInfer(_) |
            ty::TyParam(..) | ty::TyNever | ty::TyForeign(..) => false,
        }
    }

    fn visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        visitor.visit_ty(self)
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::TypeAndMut<'tcx> {
        ty, mutbl
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::GenSig<'tcx> {
        yield_ty, return_ty
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::FnSig<'tcx> {
        inputs_and_output, variadic, unsafety, abi
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::TraitRef<'tcx> { def_id, substs }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ExistentialTraitRef<'tcx> { def_id, substs }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ImplHeader<'tcx> {
        impl_def_id,
        self_ty,
        trait_ref,
        predicates,
    }
}

impl<'tcx> TypeFoldable<'tcx> for ty::Region<'tcx> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, _folder: &mut F) -> Self {
        *self
    }

    fn fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        folder.fold_region(*self)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, _visitor: &mut V) -> bool {
        false
    }

    fn visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        visitor.visit_region(*self)
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ClosureSubsts<'tcx> {
        substs,
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::GeneratorSubsts<'tcx> {
        substs,
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::adjustment::Adjustment<'tcx> {
        kind,
        target,
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::adjustment::Adjust<'tcx> {
        (ty::adjustment::Adjust::NeverToAny),
        (ty::adjustment::Adjust::ReifyFnPointer),
        (ty::adjustment::Adjust::UnsafeFnPointer),
        (ty::adjustment::Adjust::ClosureFnPointer),
        (ty::adjustment::Adjust::MutToConstPointer),
        (ty::adjustment::Adjust::Unsize),
        (ty::adjustment::Adjust::Deref)(a),
        (ty::adjustment::Adjust::Borrow)(a),
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::adjustment::OverloadedDeref<'tcx> {
        region, mutbl,
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::adjustment::AutoBorrow<'tcx> {
        (ty::adjustment::AutoBorrow::Ref)(a, b),
        (ty::adjustment::AutoBorrow::RawPtr)(m),
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::GenericPredicates<'tcx> {
        parent, predicates
    }
}

impl<'tcx> TypeFoldable<'tcx> for &'tcx ty::Slice<ty::Predicate<'tcx>> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        let v = self.iter().map(|p| p.fold_with(folder)).collect::<AccumulateVec<[_; 8]>>();
        folder.tcx().intern_predicates(&v)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.iter().any(|p| p.visit_with(visitor))
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::Predicate<'tcx> {
        (ty::Predicate::Trait)(a),
        (ty::Predicate::Subtype)(a),
        (ty::Predicate::RegionOutlives)(a),
        (ty::Predicate::TypeOutlives)(a),
        (ty::Predicate::Projection)(a),
        (ty::Predicate::WellFormed)(a),
        (ty::Predicate::ClosureKind)(a, b, c),
        (ty::Predicate::ObjectSafe)(a),
        (ty::Predicate::ConstEvaluatable)(a, b),
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ProjectionPredicate<'tcx> {
        projection_ty, ty
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ExistentialProjection<'tcx> {
        ty, substs, item_def_id
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ProjectionTy<'tcx> {
        substs, item_def_id
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::InstantiatedPredicates<'tcx> {
        predicates
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx, T> TypeFoldable<'tcx> for ty::ParamEnvAnd<'tcx, T> {
        param_env, value
    } where T: TypeFoldable<'tcx>
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::SubtypePredicate<'tcx> {
        a_is_expected, a, b
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::TraitPredicate<'tcx> {
        trait_ref
    }
}

TupleStructTypeFoldableImpl! {
    impl<'tcx,T,U> TypeFoldable<'tcx> for ty::OutlivesPredicate<T,U> {
        a, b
    } where T : TypeFoldable<'tcx>, U : TypeFoldable<'tcx>,
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::ClosureUpvar<'tcx> {
        def, span, ty
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx, T> TypeFoldable<'tcx> for ty::error::ExpectedFound<T> {
        expected, found
    } where T: TypeFoldable<'tcx>
}

impl<'tcx, T: TypeFoldable<'tcx>, I: Idx> TypeFoldable<'tcx> for IndexVec<I, T> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        self.iter().map(|x| x.fold_with(folder)).collect()
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.iter().any(|t| t.visit_with(visitor))
    }
}

EnumTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for ty::error::TypeError<'tcx> {
        (ty::error::TypeError::Mismatch),
        (ty::error::TypeError::UnsafetyMismatch)(x),
        (ty::error::TypeError::AbiMismatch)(x),
        (ty::error::TypeError::Mutability),
        (ty::error::TypeError::TupleSize)(x),
        (ty::error::TypeError::FixedArraySize)(x),
        (ty::error::TypeError::ArgCount),
        (ty::error::TypeError::RegionsDoesNotOutlive)(a, b),
        (ty::error::TypeError::RegionsInsufficientlyPolymorphic)(a, b),
        (ty::error::TypeError::RegionsOverlyPolymorphic)(a, b),
        (ty::error::TypeError::IntMismatch)(x),
        (ty::error::TypeError::FloatMismatch)(x),
        (ty::error::TypeError::Traits)(x),
        (ty::error::TypeError::VariadicMismatch)(x),
        (ty::error::TypeError::CyclicTy)(t),
        (ty::error::TypeError::ProjectionMismatched)(x),
        (ty::error::TypeError::ProjectionBoundsLength)(x),
        (ty::error::TypeError::Sorts)(x),
        (ty::error::TypeError::ExistentialMismatch)(x),
        (ty::error::TypeError::OldStyleLUB)(x),
    }
}

impl<'tcx> TypeFoldable<'tcx> for ConstVal<'tcx> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        match *self {
            ConstVal::Value(v) => ConstVal::Value(v),
            ConstVal::Unevaluated(def_id, substs) => {
                ConstVal::Unevaluated(def_id, substs.fold_with(folder))
            }
        }
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        match *self {
            ConstVal::Value(_) => false,
            ConstVal::Unevaluated(_, substs) => substs.visit_with(visitor),
        }
    }
}

impl<'tcx> TypeFoldable<'tcx> for &'tcx ty::Const<'tcx> {
    fn super_fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        let ty = self.ty.fold_with(folder);
        let val = self.val.fold_with(folder);
        folder.tcx().mk_const(ty::Const {
            ty,
            val
        })
    }

    fn fold_with<'gcx: 'tcx, F: TypeFolder<'gcx, 'tcx>>(&self, folder: &mut F) -> Self {
        folder.fold_const(*self)
    }

    fn super_visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        self.ty.visit_with(visitor) || self.val.visit_with(visitor)
    }

    fn visit_with<V: TypeVisitor<'tcx>>(&self, visitor: &mut V) -> bool {
        visitor.visit_const(self)
    }
}
