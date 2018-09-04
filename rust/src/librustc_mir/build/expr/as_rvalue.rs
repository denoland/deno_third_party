// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! See docs in build/expr/mod.rs

use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::indexed_vec::Idx;

use build::{BlockAnd, BlockAndExtension, Builder};
use build::expr::category::{Category, RvalueFunc};
use hair::*;
use rustc::middle::region;
use rustc::ty::{self, Ty, UpvarSubsts};
use rustc::mir::*;
use rustc::mir::interpret::EvalErrorKind;
use syntax_pos::Span;

impl<'a, 'gcx, 'tcx> Builder<'a, 'gcx, 'tcx> {
    /// See comment on `as_local_operand`
    pub fn as_local_rvalue<M>(&mut self, block: BasicBlock, expr: M)
                             -> BlockAnd<Rvalue<'tcx>>
        where M: Mirror<'tcx, Output = Expr<'tcx>>
    {
        let local_scope = self.local_scope();
        self.as_rvalue(block, local_scope, expr)
    }

    /// Compile `expr`, yielding an rvalue.
    pub fn as_rvalue<M>(&mut self, block: BasicBlock, scope: Option<region::Scope>, expr: M)
                        -> BlockAnd<Rvalue<'tcx>>
        where M: Mirror<'tcx, Output = Expr<'tcx>>
    {
        let expr = self.hir.mirror(expr);
        self.expr_as_rvalue(block, scope, expr)
    }

    fn expr_as_rvalue(&mut self,
                      mut block: BasicBlock,
                      scope: Option<region::Scope>,
                      expr: Expr<'tcx>)
                      -> BlockAnd<Rvalue<'tcx>> {
        debug!("expr_as_rvalue(block={:?}, scope={:?}, expr={:?})", block, scope, expr);

        let this = self;
        let expr_span = expr.span;
        let source_info = this.source_info(expr_span);

        match expr.kind {
            ExprKind::Scope { region_scope, lint_level, value } => {
                let region_scope = (region_scope, source_info);
                this.in_scope(region_scope, lint_level, block,
                              |this| this.as_rvalue(block, scope, value))
            }
            ExprKind::Repeat { value, count } => {
                let value_operand = unpack!(block = this.as_operand(block, scope, value));
                block.and(Rvalue::Repeat(value_operand, count))
            }
            ExprKind::Borrow { region, borrow_kind, arg } => {
                let arg_place = unpack!(block = this.as_place(block, arg));
                block.and(Rvalue::Ref(region, borrow_kind, arg_place))
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let lhs = unpack!(block = this.as_operand(block, scope, lhs));
                let rhs = unpack!(block = this.as_operand(block, scope, rhs));
                this.build_binary_op(block, op, expr_span, expr.ty,
                                     lhs, rhs)
            }
            ExprKind::Unary { op, arg } => {
                let arg = unpack!(block = this.as_operand(block, scope, arg));
                // Check for -MIN on signed integers
                if this.hir.check_overflow() && op == UnOp::Neg && expr.ty.is_signed() {
                    let bool_ty = this.hir.bool_ty();

                    let minval = this.minval_literal(expr_span, expr.ty);
                    let is_min = this.temp(bool_ty, expr_span);

                    this.cfg.push_assign(block, source_info, &is_min,
                                         Rvalue::BinaryOp(BinOp::Eq, arg.to_copy(), minval));

                    block = this.assert(block, Operand::Move(is_min), false,
                                        EvalErrorKind::OverflowNeg, expr_span);
                }
                block.and(Rvalue::UnaryOp(op, arg))
            }
            ExprKind::Box { value } => {
                let value = this.hir.mirror(value);
                // The `Box<T>` temporary created here is not a part of the HIR,
                // and therefore is not considered during generator OIBIT
                // determination. See the comment about `box` at `yield_in_scope`.
                let result = this.local_decls.push(
                    LocalDecl::new_internal(expr.ty, expr_span));
                this.cfg.push(block, Statement {
                    source_info,
                    kind: StatementKind::StorageLive(result)
                });
                if let Some(scope) = scope {
                    // schedule a shallow free of that memory, lest we unwind:
                    this.schedule_drop(expr_span, scope, &Place::Local(result), value.ty);
                }

                // malloc some memory of suitable type (thus far, uninitialized):
                let box_ = Rvalue::NullaryOp(NullOp::Box, value.ty);
                this.cfg.push_assign(block, source_info, &Place::Local(result), box_);

                // initialize the box contents:
                unpack!(block = this.into(&Place::Local(result).deref(), block, value));
                block.and(Rvalue::Use(Operand::Move(Place::Local(result))))
            }
            ExprKind::Cast { source } => {
                let source = this.hir.mirror(source);

                let source = unpack!(block = this.as_operand(block, scope, source));
                block.and(Rvalue::Cast(CastKind::Misc, source, expr.ty))
            }
            ExprKind::Use { source } => {
                let source = unpack!(block = this.as_operand(block, scope, source));
                block.and(Rvalue::Use(source))
            }
            ExprKind::ReifyFnPointer { source } => {
                let source = unpack!(block = this.as_operand(block, scope, source));
                block.and(Rvalue::Cast(CastKind::ReifyFnPointer, source, expr.ty))
            }
            ExprKind::UnsafeFnPointer { source } => {
                let source = unpack!(block = this.as_operand(block, scope, source));
                block.and(Rvalue::Cast(CastKind::UnsafeFnPointer, source, expr.ty))
            }
            ExprKind::ClosureFnPointer { source } => {
                let source = unpack!(block = this.as_operand(block, scope, source));
                block.and(Rvalue::Cast(CastKind::ClosureFnPointer, source, expr.ty))
            }
            ExprKind::Unsize { source } => {
                let source = unpack!(block = this.as_operand(block, scope, source));
                block.and(Rvalue::Cast(CastKind::Unsize, source, expr.ty))
            }
            ExprKind::Array { fields } => {
                // (*) We would (maybe) be closer to codegen if we
                // handled this and other aggregate cases via
                // `into()`, not `as_rvalue` -- in that case, instead
                // of generating
                //
                //     let tmp1 = ...1;
                //     let tmp2 = ...2;
                //     dest = Rvalue::Aggregate(Foo, [tmp1, tmp2])
                //
                // we could just generate
                //
                //     dest.f = ...1;
                //     dest.g = ...2;
                //
                // The problem is that then we would need to:
                //
                // (a) have a more complex mechanism for handling
                //     partial cleanup;
                // (b) distinguish the case where the type `Foo` has a
                //     destructor, in which case creating an instance
                //     as a whole "arms" the destructor, and you can't
                //     write individual fields; and,
                // (c) handle the case where the type Foo has no
                //     fields. We don't want `let x: ();` to compile
                //     to the same MIR as `let x = ();`.

                // first process the set of fields
                let el_ty = expr.ty.sequence_element_type(this.hir.tcx());
                let fields: Vec<_> =
                    fields.into_iter()
                          .map(|f| unpack!(block = this.as_operand(block, scope, f)))
                          .collect();

                block.and(Rvalue::Aggregate(box AggregateKind::Array(el_ty), fields))
            }
            ExprKind::Tuple { fields } => { // see (*) above
                // first process the set of fields
                let fields: Vec<_> =
                    fields.into_iter()
                          .map(|f| unpack!(block = this.as_operand(block, scope, f)))
                          .collect();

                block.and(Rvalue::Aggregate(box AggregateKind::Tuple, fields))
            }
            ExprKind::Closure { closure_id, substs, upvars, movability } => {
                // see (*) above
                let mut operands: Vec<_> =
                    upvars.into_iter()
                          .map(|upvar| unpack!(block = this.as_operand(block, scope, upvar)))
                          .collect();
                let result = match substs {
                    UpvarSubsts::Generator(substs) => {
                        let movability = movability.unwrap();
                        // Add the state operand since it follows the upvars in the generator
                        // struct. See librustc_mir/transform/generator.rs for more details.
                        operands.push(Operand::Constant(box Constant {
                            span: expr_span,
                            ty: this.hir.tcx().types.u32,
                            literal: Literal::Value {
                                value: ty::Const::from_bits(
                                    this.hir.tcx(),
                                    0,
                                    ty::ParamEnv::empty().and(this.hir.tcx().types.u32)),
                            },
                        }));
                        box AggregateKind::Generator(closure_id, substs, movability)
                    }
                    UpvarSubsts::Closure(substs) => {
                        box AggregateKind::Closure(closure_id, substs)
                    }
                };
                block.and(Rvalue::Aggregate(result, operands))
            }
            ExprKind::Adt {
                adt_def, variant_index, substs, fields, base
            } => { // see (*) above
                let is_union = adt_def.is_union();
                let active_field_index = if is_union { Some(fields[0].name.index()) } else { None };

                // first process the set of fields that were provided
                // (evaluating them in order given by user)
                let fields_map: FxHashMap<_, _> = fields.into_iter()
                    .map(|f| (f.name, unpack!(block = this.as_operand(block, scope, f.expr))))
                    .collect();

                let field_names = this.hir.all_fields(adt_def, variant_index);

                let fields = if let Some(FruInfo { base, field_types }) = base {
                    let base = unpack!(block = this.as_place(block, base));

                    // MIR does not natively support FRU, so for each
                    // base-supplied field, generate an operand that
                    // reads it from the base.
                    field_names.into_iter()
                        .zip(field_types.into_iter())
                        .map(|(n, ty)| match fields_map.get(&n) {
                            Some(v) => v.clone(),
                            None => this.consume_by_copy_or_move(base.clone().field(n, ty))
                        })
                        .collect()
                } else {
                    field_names.iter().filter_map(|n| fields_map.get(n).cloned()).collect()
                };

                let adt =
                    box AggregateKind::Adt(adt_def, variant_index, substs, active_field_index);
                block.and(Rvalue::Aggregate(adt, fields))
            }
            ExprKind::Assign { .. } |
            ExprKind::AssignOp { .. } => {
                block = unpack!(this.stmt_expr(block, expr));
                block.and(this.unit_rvalue())
            }
            ExprKind::Yield { value } => {
                let value = unpack!(block = this.as_operand(block, scope, value));
                let resume = this.cfg.start_new_block();
                let cleanup = this.generator_drop_cleanup();
                this.cfg.terminate(block, source_info, TerminatorKind::Yield {
                    value: value,
                    resume: resume,
                    drop: cleanup,
                });
                resume.and(this.unit_rvalue())
            }
            ExprKind::Literal { .. } |
            ExprKind::Block { .. } |
            ExprKind::Match { .. } |
            ExprKind::If { .. } |
            ExprKind::NeverToAny { .. } |
            ExprKind::Loop { .. } |
            ExprKind::LogicalOp { .. } |
            ExprKind::Call { .. } |
            ExprKind::Field { .. } |
            ExprKind::Deref { .. } |
            ExprKind::Index { .. } |
            ExprKind::VarRef { .. } |
            ExprKind::SelfRef |
            ExprKind::Break { .. } |
            ExprKind::Continue { .. } |
            ExprKind::Return { .. } |
            ExprKind::InlineAsm { .. } |
            ExprKind::StaticRef { .. } => {
                // these do not have corresponding `Rvalue` variants,
                // so make an operand and then return that
                debug_assert!(match Category::of(&expr.kind) {
                    Some(Category::Rvalue(RvalueFunc::AsRvalue)) => false,
                    _ => true,
                });
                let operand = unpack!(block = this.as_operand(block, scope, expr));
                block.and(Rvalue::Use(operand))
            }
        }
    }

    pub fn build_binary_op(&mut self, mut block: BasicBlock,
                           op: BinOp, span: Span, ty: Ty<'tcx>,
                           lhs: Operand<'tcx>, rhs: Operand<'tcx>) -> BlockAnd<Rvalue<'tcx>> {
        let source_info = self.source_info(span);
        let bool_ty = self.hir.bool_ty();
        if self.hir.check_overflow() && op.is_checkable() && ty.is_integral() {
            let result_tup = self.hir.tcx().intern_tup(&[ty, bool_ty]);
            let result_value = self.temp(result_tup, span);

            self.cfg.push_assign(block, source_info,
                                 &result_value, Rvalue::CheckedBinaryOp(op,
                                                                        lhs,
                                                                        rhs));
            let val_fld = Field::new(0);
            let of_fld = Field::new(1);

            let val = result_value.clone().field(val_fld, ty);
            let of = result_value.field(of_fld, bool_ty);

            let err = EvalErrorKind::Overflow(op);

            block = self.assert(block, Operand::Move(of), false,
                                err, span);

            block.and(Rvalue::Use(Operand::Move(val)))
        } else {
            if ty.is_integral() && (op == BinOp::Div || op == BinOp::Rem) {
                // Checking division and remainder is more complex, since we 1. always check
                // and 2. there are two possible failure cases, divide-by-zero and overflow.

                let (zero_err, overflow_err) = if op == BinOp::Div {
                    (EvalErrorKind::DivisionByZero,
                     EvalErrorKind::Overflow(op))
                } else {
                    (EvalErrorKind::RemainderByZero,
                     EvalErrorKind::Overflow(op))
                };

                // Check for / 0
                let is_zero = self.temp(bool_ty, span);
                let zero = self.zero_literal(span, ty);
                self.cfg.push_assign(block, source_info, &is_zero,
                                     Rvalue::BinaryOp(BinOp::Eq, rhs.to_copy(), zero));

                block = self.assert(block, Operand::Move(is_zero), false,
                                    zero_err, span);

                // We only need to check for the overflow in one case:
                // MIN / -1, and only for signed values.
                if ty.is_signed() {
                    let neg_1 = self.neg_1_literal(span, ty);
                    let min = self.minval_literal(span, ty);

                    let is_neg_1 = self.temp(bool_ty, span);
                    let is_min   = self.temp(bool_ty, span);
                    let of       = self.temp(bool_ty, span);

                    // this does (rhs == -1) & (lhs == MIN). It could short-circuit instead

                    self.cfg.push_assign(block, source_info, &is_neg_1,
                                         Rvalue::BinaryOp(BinOp::Eq, rhs.to_copy(), neg_1));
                    self.cfg.push_assign(block, source_info, &is_min,
                                         Rvalue::BinaryOp(BinOp::Eq, lhs.to_copy(), min));

                    let is_neg_1 = Operand::Move(is_neg_1);
                    let is_min = Operand::Move(is_min);
                    self.cfg.push_assign(block, source_info, &of,
                                         Rvalue::BinaryOp(BinOp::BitAnd, is_neg_1, is_min));

                    block = self.assert(block, Operand::Move(of), false,
                                        overflow_err, span);
                }
            }

            block.and(Rvalue::BinaryOp(op, lhs, rhs))
        }
    }

    // Helper to get a `-1` value of the appropriate type
    fn neg_1_literal(&mut self, span: Span, ty: Ty<'tcx>) -> Operand<'tcx> {
        let param_ty = ty::ParamEnv::empty().and(self.hir.tcx().lift_to_global(&ty).unwrap());
        let bits = self.hir.tcx().layout_of(param_ty).unwrap().size.bits();
        let n = (!0u128) >> (128 - bits);
        let literal = Literal::Value {
            value: ty::Const::from_bits(self.hir.tcx(), n, param_ty)
        };

        self.literal_operand(span, ty, literal)
    }

    // Helper to get the minimum value of the appropriate type
    fn minval_literal(&mut self, span: Span, ty: Ty<'tcx>) -> Operand<'tcx> {
        assert!(ty.is_signed());
        let param_ty = ty::ParamEnv::empty().and(self.hir.tcx().lift_to_global(&ty).unwrap());
        let bits = self.hir.tcx().layout_of(param_ty).unwrap().size.bits();
        let n = 1 << (bits - 1);
        let literal = Literal::Value {
            value: ty::Const::from_bits(self.hir.tcx(), n, param_ty)
        };

        self.literal_operand(span, ty, literal)
    }
}
