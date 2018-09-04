// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Propagates constants for early reporting of statically known
//! assertion failures


use rustc::hir::def::Def;
use rustc::mir::{Constant, Literal, Location, Place, Mir, Operand, Rvalue, Local};
use rustc::mir::{NullOp, StatementKind, Statement, BasicBlock, LocalKind};
use rustc::mir::{TerminatorKind, ClearCrossCrate, SourceInfo, BinOp, ProjectionElem};
use rustc::mir::visit::{Visitor, PlaceContext};
use rustc::middle::const_val::{ConstVal, ConstEvalErr, ErrKind};
use rustc::ty::{TyCtxt, self, Instance};
use rustc::mir::interpret::{Value, Scalar, GlobalId, EvalResult};
use interpret::EvalContext;
use interpret::CompileTimeEvaluator;
use interpret::{eval_promoted, mk_borrowck_eval_cx, ValTy};
use transform::{MirPass, MirSource};
use syntax::codemap::{Span, DUMMY_SP};
use rustc::ty::subst::Substs;
use rustc_data_structures::indexed_vec::IndexVec;
use rustc::ty::ParamEnv;
use rustc::ty::layout::{
    LayoutOf, TyLayout, LayoutError,
    HasTyCtxt, TargetDataLayout, HasDataLayout,
};

pub struct ConstProp;

impl MirPass for ConstProp {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          source: MirSource,
                          mir: &mut Mir<'tcx>) {
        // will be evaluated by miri and produce its errors there
        if source.promoted.is_some() {
            return;
        }
        match tcx.describe_def(source.def_id) {
            // skip statics because they'll be evaluated by miri anyway
            Some(Def::Static(..)) => return,
            _ => {},
        }
        trace!("ConstProp starting for {:?}", source.def_id);

        // FIXME(oli-obk, eddyb) Optimize locals (or even local paths) to hold
        // constants, instead of just checking for const-folding succeeding.
        // That would require an uniform one-def no-mutation analysis
        // and RPO (or recursing when needing the value of a local).
        let mut optimization_finder = ConstPropagator::new(mir, tcx, source);
        optimization_finder.visit_mir(mir);

        trace!("ConstProp done for {:?}", source.def_id);
    }
}

type Const<'tcx> = (Value, ty::Ty<'tcx>, Span);

/// Finds optimization opportunities on the MIR.
struct ConstPropagator<'b, 'a, 'tcx:'a+'b> {
    ecx: EvalContext<'a, 'b, 'tcx, CompileTimeEvaluator>,
    mir: &'b Mir<'tcx>,
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    source: MirSource,
    places: IndexVec<Local, Option<Const<'tcx>>>,
    can_const_prop: IndexVec<Local, bool>,
    param_env: ParamEnv<'tcx>,
}

impl<'a, 'b, 'tcx> LayoutOf for &'a ConstPropagator<'a, 'b, 'tcx> {
    type Ty = ty::Ty<'tcx>;
    type TyLayout = Result<TyLayout<'tcx>, LayoutError<'tcx>>;

    fn layout_of(self, ty: ty::Ty<'tcx>) -> Self::TyLayout {
        self.tcx.layout_of(self.param_env.and(ty))
    }
}

impl<'a, 'b, 'tcx> HasDataLayout for &'a ConstPropagator<'a, 'b, 'tcx> {
    #[inline]
    fn data_layout(&self) -> &TargetDataLayout {
        &self.tcx.data_layout
    }
}

impl<'a, 'b, 'tcx> HasTyCtxt<'tcx> for &'a ConstPropagator<'a, 'b, 'tcx> {
    #[inline]
    fn tcx<'c>(&'c self) -> TyCtxt<'c, 'tcx, 'tcx> {
        self.tcx
    }
}

impl<'b, 'a, 'tcx:'b> ConstPropagator<'b, 'a, 'tcx> {
    fn new(
        mir: &'b Mir<'tcx>,
        tcx: TyCtxt<'a, 'tcx, 'tcx>,
        source: MirSource,
    ) -> ConstPropagator<'b, 'a, 'tcx> {
        let param_env = tcx.param_env(source.def_id);
        let substs = Substs::identity_for_item(tcx, source.def_id);
        let instance = Instance::new(source.def_id, substs);
        let ecx = mk_borrowck_eval_cx(tcx, instance, mir, DUMMY_SP).unwrap();
        ConstPropagator {
            ecx,
            mir,
            tcx,
            source,
            param_env,
            can_const_prop: CanConstProp::check(mir),
            places: IndexVec::from_elem(None, &mir.local_decls),
        }
    }

    fn use_ecx<F, T>(
        &mut self,
        source_info: SourceInfo,
        f: F
    ) -> Option<T>
    where
        F: FnOnce(&mut Self) -> EvalResult<'tcx, T>,
    {
        self.ecx.tcx.span = source_info.span;
        let lint_root = match self.mir.source_scope_local_data {
            ClearCrossCrate::Set(ref ivs) => {
                use rustc_data_structures::indexed_vec::Idx;
                //FIXME(#51314): remove this check
                if source_info.scope.index() >= ivs.len() {
                    return None;
                }
                ivs[source_info.scope].lint_root
            },
            ClearCrossCrate::Clear => return None,
        };
        let r = match f(self) {
            Ok(val) => Some(val),
            Err(err) => {
                let (frames, span) = self.ecx.generate_stacktrace(None);
                let err = ConstEvalErr {
                    span,
                    kind: ErrKind::Miri(err, frames).into(),
                };
                err.report_as_lint(
                    self.ecx.tcx,
                    "this expression will panic at runtime",
                    lint_root,
                );
                None
            },
        };
        self.ecx.tcx.span = DUMMY_SP;
        r
    }

    fn const_eval(&mut self, cid: GlobalId<'tcx>, source_info: SourceInfo) -> Option<Const<'tcx>> {
        let value = match self.tcx.const_eval(self.param_env.and(cid)) {
            Ok(val) => val,
            Err(err) => {
                err.report_as_error(
                    self.tcx.at(err.span),
                    "constant evaluation error",
                );
                return None;
            },
        };
        let val = match value.val {
            ConstVal::Value(v) => {
                self.use_ecx(source_info, |this| this.ecx.const_value_to_value(v, value.ty))?
            },
            _ => bug!("eval produced: {:?}", value),
        };
        let val = (val, value.ty, source_info.span);
        trace!("evaluated {:?} to {:?}", cid, val);
        Some(val)
    }

    fn eval_constant(
        &mut self,
        c: &Constant<'tcx>,
        source_info: SourceInfo,
    ) -> Option<Const<'tcx>> {
        match c.literal {
            Literal::Value { value } => match value.val {
                ConstVal::Value(v) => {
                    let v = self.use_ecx(source_info, |this| {
                        this.ecx.const_value_to_value(v, value.ty)
                    })?;
                    Some((v, value.ty, c.span))
                },
                ConstVal::Unevaluated(did, substs) => {
                    let instance = Instance::resolve(
                        self.tcx,
                        self.param_env,
                        did,
                        substs,
                    )?;
                    let cid = GlobalId {
                        instance,
                        promoted: None,
                    };
                    self.const_eval(cid, source_info)
                },
            },
            // evaluate the promoted and replace the constant with the evaluated result
            Literal::Promoted { index } => {
                let generics = self.tcx.generics_of(self.source.def_id);
                if generics.requires_monomorphization(self.tcx) {
                    // FIXME: can't handle code with generics
                    return None;
                }
                let substs = Substs::identity_for_item(self.tcx, self.source.def_id);
                let instance = Instance::new(self.source.def_id, substs);
                let cid = GlobalId {
                    instance,
                    promoted: Some(index),
                };
                // cannot use `const_eval` here, because that would require having the MIR
                // for the current function available, but we're producing said MIR right now
                let (value, _, ty) = self.use_ecx(source_info, |this| {
                    eval_promoted(&mut this.ecx, cid, this.mir, this.param_env)
                })?;
                let val = (value, ty, c.span);
                trace!("evaluated {:?} to {:?}", c, val);
                Some(val)
            }
        }
    }

    fn eval_place(&mut self, place: &Place<'tcx>, source_info: SourceInfo) -> Option<Const<'tcx>> {
        match *place {
            Place::Local(loc) => self.places[loc].clone(),
            Place::Projection(ref proj) => match proj.elem {
                ProjectionElem::Field(field, _) => {
                    trace!("field proj on {:?}", proj.base);
                    let (base, ty, span) = self.eval_place(&proj.base, source_info)?;
                    let valty = self.use_ecx(source_info, |this| {
                        this.ecx.read_field(base, None, field, ty)
                    })?;
                    Some((valty.value, valty.ty, span))
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn eval_operand(&mut self, op: &Operand<'tcx>, source_info: SourceInfo) -> Option<Const<'tcx>> {
        match *op {
            Operand::Constant(ref c) => self.eval_constant(c, source_info),
            | Operand::Move(ref place)
            | Operand::Copy(ref place) => self.eval_place(place, source_info),
        }
    }

    fn const_prop(
        &mut self,
        rvalue: &Rvalue<'tcx>,
        place_ty: ty::Ty<'tcx>,
        source_info: SourceInfo,
    ) -> Option<Const<'tcx>> {
        let span = source_info.span;
        match *rvalue {
            // This branch exists for the sanity type check
            Rvalue::Use(Operand::Constant(ref c)) => {
                assert_eq!(c.ty, place_ty);
                self.eval_constant(c, source_info)
            },
            Rvalue::Use(ref op) => {
                self.eval_operand(op, source_info)
            },
            Rvalue::Repeat(..) |
            Rvalue::Ref(..) |
            Rvalue::Cast(..) |
            Rvalue::Aggregate(..) |
            Rvalue::NullaryOp(NullOp::Box, _) |
            Rvalue::Discriminant(..) => None,
            // FIXME(oli-obk): evaluate static/constant slice lengths
            Rvalue::Len(_) => None,
            Rvalue::NullaryOp(NullOp::SizeOf, ty) => {
                let param_env = self.tcx.param_env(self.source.def_id);
                type_size_of(self.tcx, param_env, ty).map(|n| (
                    Value::Scalar(Scalar::Bits {
                        bits: n as u128,
                        defined: self.tcx.data_layout.pointer_size.bits() as u8,
                    }),
                    self.tcx.types.usize,
                    span,
                ))
            }
            Rvalue::UnaryOp(op, ref arg) => {
                let def_id = if self.tcx.is_closure(self.source.def_id) {
                    self.tcx.closure_base_def_id(self.source.def_id)
                } else {
                    self.source.def_id
                };
                let generics = self.tcx.generics_of(def_id);
                if generics.requires_monomorphization(self.tcx) {
                    // FIXME: can't handle code with generics
                    return None;
                }

                let val = self.eval_operand(arg, source_info)?;
                let prim = self.use_ecx(source_info, |this| {
                    this.ecx.value_to_scalar(ValTy { value: val.0, ty: val.1 })
                })?;
                let val = self.use_ecx(source_info, |this| this.ecx.unary_op(op, prim, val.1))?;
                Some((Value::Scalar(val), place_ty, span))
            }
            Rvalue::CheckedBinaryOp(op, ref left, ref right) |
            Rvalue::BinaryOp(op, ref left, ref right) => {
                trace!("rvalue binop {:?} for {:?} and {:?}", op, left, right);
                let right = self.eval_operand(right, source_info)?;
                let def_id = if self.tcx.is_closure(self.source.def_id) {
                    self.tcx.closure_base_def_id(self.source.def_id)
                } else {
                    self.source.def_id
                };
                let generics = self.tcx.generics_of(def_id);
                if generics.requires_monomorphization(self.tcx) {
                    // FIXME: can't handle code with generics
                    return None;
                }

                let r = self.use_ecx(source_info, |this| {
                    this.ecx.value_to_scalar(ValTy { value: right.0, ty: right.1 })
                })?;
                if op == BinOp::Shr || op == BinOp::Shl {
                    let left_ty = left.ty(self.mir, self.tcx);
                    let left_bits = self
                        .tcx
                        .layout_of(self.param_env.and(left_ty))
                        .unwrap()
                        .size
                        .bits();
                    let right_size = self.tcx.layout_of(self.param_env.and(right.1)).unwrap().size;
                    if r.to_bits(right_size).ok().map_or(false, |b| b >= left_bits as u128) {
                        let source_scope_local_data = match self.mir.source_scope_local_data {
                            ClearCrossCrate::Set(ref data) => data,
                            ClearCrossCrate::Clear => return None,
                        };
                        let dir = if op == BinOp::Shr {
                            "right"
                        } else {
                            "left"
                        };
                        let node_id = source_scope_local_data[source_info.scope].lint_root;
                        self.tcx.lint_node(
                            ::rustc::lint::builtin::EXCEEDING_BITSHIFTS,
                            node_id,
                            span,
                            &format!("attempt to shift {} with overflow", dir));
                        return None;
                    }
                }
                let left = self.eval_operand(left, source_info)?;
                let l = self.use_ecx(source_info, |this| {
                    this.ecx.value_to_scalar(ValTy { value: left.0, ty: left.1 })
                })?;
                trace!("const evaluating {:?} for {:?} and {:?}", op, left, right);
                let (val, overflow) = self.use_ecx(source_info, |this| {
                    this.ecx.binary_op(op, l, left.1, r, right.1)
                })?;
                let val = if let Rvalue::CheckedBinaryOp(..) = *rvalue {
                    Value::ScalarPair(
                        val,
                        Scalar::from_bool(overflow),
                    )
                } else {
                    if overflow {
                        use rustc::mir::interpret::EvalErrorKind;
                        let err = EvalErrorKind::Overflow(op).into();
                        let _: Option<()> = self.use_ecx(source_info, |_| Err(err));
                        return None;
                    }
                    Value::Scalar(val)
                };
                Some((val, place_ty, span))
            },
        }
    }
}

fn type_size_of<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          param_env: ty::ParamEnv<'tcx>,
                          ty: ty::Ty<'tcx>) -> Option<u64> {
    tcx.layout_of(param_env.and(ty)).ok().map(|layout| layout.size.bytes())
}

struct CanConstProp {
    can_const_prop: IndexVec<Local, bool>,
    // false at the beginning, once set, there are not allowed to be any more assignments
    found_assignment: IndexVec<Local, bool>,
}

impl CanConstProp {
    /// returns true if `local` can be propagated
    fn check(mir: &Mir) -> IndexVec<Local, bool> {
        let mut cpv = CanConstProp {
            can_const_prop: IndexVec::from_elem(true, &mir.local_decls),
            found_assignment: IndexVec::from_elem(false, &mir.local_decls),
        };
        for (local, val) in cpv.can_const_prop.iter_enumerated_mut() {
            // cannot use args at all
            // cannot use locals because if x < y { y - x } else { x - y } would
            //        lint for x != y
            // FIXME(oli-obk): lint variables until they are used in a condition
            // FIXME(oli-obk): lint if return value is constant
            *val = mir.local_kind(local) == LocalKind::Temp;
        }
        cpv.visit_mir(mir);
        cpv.can_const_prop
    }
}

impl<'tcx> Visitor<'tcx> for CanConstProp {
    fn visit_local(
        &mut self,
        &local: &Local,
        context: PlaceContext<'tcx>,
        _: Location,
    ) {
        use rustc::mir::visit::PlaceContext::*;
        match context {
            // Constants must have at most one write
            // FIXME(oli-obk): we could be more powerful here, if the multiple writes
            // only occur in independent execution paths
            Store => if self.found_assignment[local] {
                self.can_const_prop[local] = false;
            } else {
                self.found_assignment[local] = true
            },
            // Reading constants is allowed an arbitrary number of times
            Copy | Move |
            StorageDead | StorageLive |
            Validate |
            Projection(_) |
            Inspect => {},
            _ => self.can_const_prop[local] = false,
        }
    }
}

impl<'b, 'a, 'tcx> Visitor<'tcx> for ConstPropagator<'b, 'a, 'tcx> {
    fn visit_constant(
        &mut self,
        constant: &Constant<'tcx>,
        location: Location,
    ) {
        trace!("visit_constant: {:?}", constant);
        self.super_constant(constant, location);
        let source_info = *self.mir.source_info(location);
        self.eval_constant(constant, source_info);
    }

    fn visit_statement(
        &mut self,
        block: BasicBlock,
        statement: &Statement<'tcx>,
        location: Location,
    ) {
        trace!("visit_statement: {:?}", statement);
        if let StatementKind::Assign(ref place, ref rval) = statement.kind {
            let place_ty = place
                .ty(&self.mir.local_decls, self.tcx)
                .to_ty(self.tcx);
            if let Some(value) = self.const_prop(rval, place_ty, statement.source_info) {
                if let Place::Local(local) = *place {
                    trace!("checking whether {:?} can be stored to {:?}", value, local);
                    if self.can_const_prop[local] {
                        trace!("storing {:?} to {:?}", value, local);
                        assert!(self.places[local].is_none());
                        self.places[local] = Some(value);
                    }
                }
            }
        }
        self.super_statement(block, statement, location);
    }

    fn visit_terminator_kind(
        &mut self,
        block: BasicBlock,
        kind: &TerminatorKind<'tcx>,
        location: Location,
    ) {
        self.super_terminator_kind(block, kind, location);
        let source_info = *self.mir.source_info(location);
        if let TerminatorKind::Assert { expected, msg, cond, .. } = kind {
            if let Some(value) = self.eval_operand(cond, source_info) {
                trace!("assertion on {:?} should be {:?}", value, expected);
                if Value::Scalar(Scalar::from_bool(*expected)) != value.0 {
                    // poison all places this operand references so that further code
                    // doesn't use the invalid value
                    match cond {
                        Operand::Move(ref place) | Operand::Copy(ref place) => {
                            let mut place = place;
                            while let Place::Projection(ref proj) = *place {
                                place = &proj.base;
                            }
                            if let Place::Local(local) = *place {
                                self.places[local] = None;
                            }
                        },
                        Operand::Constant(_) => {}
                    }
                    let span = self.mir[block]
                        .terminator
                        .as_ref()
                        .unwrap()
                        .source_info
                        .span;
                    let node_id = self
                        .tcx
                        .hir
                        .as_local_node_id(self.source.def_id)
                        .expect("some part of a failing const eval must be local");
                    use rustc::mir::interpret::EvalErrorKind::*;
                    let msg = match msg {
                        Overflow(_) |
                        OverflowNeg |
                        DivisionByZero |
                        RemainderByZero => msg.description().to_owned(),
                        BoundsCheck { ref len, ref index } => {
                            let len = self
                                .eval_operand(len, source_info)
                                .expect("len must be const");
                            let len = match len.0 {
                                Value::Scalar(Scalar::Bits { bits, ..}) => bits,
                                _ => bug!("const len not primitive: {:?}", len),
                            };
                            let index = self
                                .eval_operand(index, source_info)
                                .expect("index must be const");
                            let index = match index.0 {
                                Value::Scalar(Scalar::Bits { bits, .. }) => bits,
                                _ => bug!("const index not primitive: {:?}", index),
                            };
                            format!(
                                "index out of bounds: \
                                the len is {} but the index is {}",
                                len,
                                index,
                            )
                        },
                        // Need proper const propagator for these
                        _ => return,
                    };
                    self.tcx.lint_node(
                        ::rustc::lint::builtin::CONST_ERR,
                        node_id,
                        span,
                        &msg,
                    );
                }
            }
        }
    }
}
