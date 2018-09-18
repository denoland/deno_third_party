// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to validate patterns/matches

mod _match;
mod check_match;

pub use self::check_match::check_crate;
pub(crate) use self::check_match::check_match;

use interpret::{const_val_field, const_variant_index, self};

use rustc::middle::const_val::ConstVal;
use rustc::mir::{fmt_const_val, Field, BorrowKind, Mutability};
use rustc::mir::interpret::{Scalar, GlobalId, ConstValue, Value};
use rustc::ty::{self, TyCtxt, AdtDef, Ty, Region};
use rustc::ty::subst::{Substs, Kind};
use rustc::hir::{self, PatKind, RangeEnd};
use rustc::hir::def::{Def, CtorKind};
use rustc::hir::pat_util::EnumerateAndAdjustIterator;

use rustc_data_structures::indexed_vec::Idx;

use std::cmp::Ordering;
use std::fmt;
use syntax::ast;
use syntax::ptr::P;
use syntax_pos::Span;
use syntax_pos::symbol::Symbol;

#[derive(Clone, Debug)]
pub enum PatternError {
    AssociatedConstInPattern(Span),
    StaticInPattern(Span),
    FloatBug,
    NonConstPath(Span),
}

#[derive(Copy, Clone, Debug)]
pub enum BindingMode<'tcx> {
    ByValue,
    ByRef(Region<'tcx>, BorrowKind),
}

#[derive(Clone, Debug)]
pub struct FieldPattern<'tcx> {
    pub field: Field,
    pub pattern: Pattern<'tcx>,
}

#[derive(Clone, Debug)]
pub struct Pattern<'tcx> {
    pub ty: Ty<'tcx>,
    pub span: Span,
    pub kind: Box<PatternKind<'tcx>>,
}

#[derive(Clone, Debug)]
pub enum PatternKind<'tcx> {
    Wild,

    /// x, ref x, x @ P, etc
    Binding {
        mutability: Mutability,
        name: ast::Name,
        mode: BindingMode<'tcx>,
        var: ast::NodeId,
        ty: Ty<'tcx>,
        subpattern: Option<Pattern<'tcx>>,
    },

    /// Foo(...) or Foo{...} or Foo, where `Foo` is a variant name from an adt with >1 variants
    Variant {
        adt_def: &'tcx AdtDef,
        substs: &'tcx Substs<'tcx>,
        variant_index: usize,
        subpatterns: Vec<FieldPattern<'tcx>>,
    },

    /// (...), Foo(...), Foo{...}, or Foo, where `Foo` is a variant name from an adt with 1 variant
    Leaf {
        subpatterns: Vec<FieldPattern<'tcx>>,
    },

    /// box P, &P, &mut P, etc
    Deref {
        subpattern: Pattern<'tcx>,
    },

    Constant {
        value: &'tcx ty::Const<'tcx>,
    },

    Range {
        lo: &'tcx ty::Const<'tcx>,
        hi: &'tcx ty::Const<'tcx>,
        end: RangeEnd,
    },

    /// matches against a slice, checking the length and extracting elements.
    /// irrefutable when there is a slice pattern and both `prefix` and `suffix` are empty.
    /// e.g. `&[ref xs..]`.
    Slice {
        prefix: Vec<Pattern<'tcx>>,
        slice: Option<Pattern<'tcx>>,
        suffix: Vec<Pattern<'tcx>>,
    },

    /// fixed match against an array, irrefutable
    Array {
        prefix: Vec<Pattern<'tcx>>,
        slice: Option<Pattern<'tcx>>,
        suffix: Vec<Pattern<'tcx>>,
    },
}

fn print_const_val(value: &ty::Const, f: &mut fmt::Formatter) -> fmt::Result {
    match value.val {
        ConstVal::Value(..) => fmt_const_val(f, value),
        ConstVal::Unevaluated(..) => bug!("{:?} not printable in a pattern", value)
    }
}

impl<'tcx> fmt::Display for Pattern<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.kind {
            PatternKind::Wild => write!(f, "_"),
            PatternKind::Binding { mutability, name, mode, ref subpattern, .. } => {
                let is_mut = match mode {
                    BindingMode::ByValue => mutability == Mutability::Mut,
                    BindingMode::ByRef(_, bk) => {
                        write!(f, "ref ")?;
                        match bk { BorrowKind::Mut { .. } => true, _ => false }
                    }
                };
                if is_mut {
                    write!(f, "mut ")?;
                }
                write!(f, "{}", name)?;
                if let Some(ref subpattern) = *subpattern {
                    write!(f, " @ {}", subpattern)?;
                }
                Ok(())
            }
            PatternKind::Variant { ref subpatterns, .. } |
            PatternKind::Leaf { ref subpatterns } => {
                let variant = match *self.kind {
                    PatternKind::Variant { adt_def, variant_index, .. } => {
                        Some(&adt_def.variants[variant_index])
                    }
                    _ => if let ty::TyAdt(adt, _) = self.ty.sty {
                        if !adt.is_enum() {
                            Some(&adt.variants[0])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                let mut first = true;
                let mut start_or_continue = || if first { first = false; "" } else { ", " };

                if let Some(variant) = variant {
                    write!(f, "{}", variant.name)?;

                    // Only for TyAdt we can have `S {...}`,
                    // which we handle separately here.
                    if variant.ctor_kind == CtorKind::Fictive {
                        write!(f, " {{ ")?;

                        let mut printed = 0;
                        for p in subpatterns {
                            if let PatternKind::Wild = *p.pattern.kind {
                                continue;
                            }
                            let name = variant.fields[p.field.index()].ident;
                            write!(f, "{}{}: {}", start_or_continue(), name, p.pattern)?;
                            printed += 1;
                        }

                        if printed < variant.fields.len() {
                            write!(f, "{}..", start_or_continue())?;
                        }

                        return write!(f, " }}");
                    }
                }

                let num_fields = variant.map_or(subpatterns.len(), |v| v.fields.len());
                if num_fields != 0 || variant.is_none() {
                    write!(f, "(")?;
                    for i in 0..num_fields {
                        write!(f, "{}", start_or_continue())?;

                        // Common case: the field is where we expect it.
                        if let Some(p) = subpatterns.get(i) {
                            if p.field.index() == i {
                                write!(f, "{}", p.pattern)?;
                                continue;
                            }
                        }

                        // Otherwise, we have to go looking for it.
                        if let Some(p) = subpatterns.iter().find(|p| p.field.index() == i) {
                            write!(f, "{}", p.pattern)?;
                        } else {
                            write!(f, "_")?;
                        }
                    }
                    write!(f, ")")?;
                }

                Ok(())
            }
            PatternKind::Deref { ref subpattern } => {
                match self.ty.sty {
                    ty::TyAdt(def, _) if def.is_box() => write!(f, "box ")?,
                    ty::TyRef(_, _, mutbl) => {
                        write!(f, "&")?;
                        if mutbl == hir::MutMutable {
                            write!(f, "mut ")?;
                        }
                    }
                    _ => bug!("{} is a bad Deref pattern type", self.ty)
                }
                write!(f, "{}", subpattern)
            }
            PatternKind::Constant { value } => {
                print_const_val(value, f)
            }
            PatternKind::Range { lo, hi, end } => {
                print_const_val(lo, f)?;
                match end {
                    RangeEnd::Included => write!(f, "...")?,
                    RangeEnd::Excluded => write!(f, "..")?,
                }
                print_const_val(hi, f)
            }
            PatternKind::Slice { ref prefix, ref slice, ref suffix } |
            PatternKind::Array { ref prefix, ref slice, ref suffix } => {
                let mut first = true;
                let mut start_or_continue = || if first { first = false; "" } else { ", " };
                write!(f, "[")?;
                for p in prefix {
                    write!(f, "{}{}", start_or_continue(), p)?;
                }
                if let Some(ref slice) = *slice {
                    write!(f, "{}", start_or_continue())?;
                    match *slice.kind {
                        PatternKind::Wild => {}
                        _ => write!(f, "{}", slice)?
                    }
                    write!(f, "..")?;
                }
                for p in suffix {
                    write!(f, "{}{}", start_or_continue(), p)?;
                }
                write!(f, "]")
            }
        }
    }
}

pub struct PatternContext<'a, 'tcx: 'a> {
    pub tcx: TyCtxt<'a, 'tcx, 'tcx>,
    pub param_env: ty::ParamEnv<'tcx>,
    pub tables: &'a ty::TypeckTables<'tcx>,
    pub substs: &'tcx Substs<'tcx>,
    pub errors: Vec<PatternError>,
}

impl<'a, 'tcx> Pattern<'tcx> {
    pub fn from_hir(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                    param_env_and_substs: ty::ParamEnvAnd<'tcx, &'tcx Substs<'tcx>>,
                    tables: &'a ty::TypeckTables<'tcx>,
                    pat: &'tcx hir::Pat) -> Self {
        let mut pcx = PatternContext::new(tcx, param_env_and_substs, tables);
        let result = pcx.lower_pattern(pat);
        if !pcx.errors.is_empty() {
            let msg = format!("encountered errors lowering pattern: {:?}", pcx.errors);
            tcx.sess.delay_span_bug(pat.span, &msg);
        }
        debug!("Pattern::from_hir({:?}) = {:?}", pat, result);
        result
    }
}

impl<'a, 'tcx> PatternContext<'a, 'tcx> {
    pub fn new(tcx: TyCtxt<'a, 'tcx, 'tcx>,
               param_env_and_substs: ty::ParamEnvAnd<'tcx, &'tcx Substs<'tcx>>,
               tables: &'a ty::TypeckTables<'tcx>) -> Self {
        PatternContext {
            tcx,
            param_env: param_env_and_substs.param_env,
            tables,
            substs: param_env_and_substs.value,
            errors: vec![]
        }
    }

    pub fn lower_pattern(&mut self, pat: &'tcx hir::Pat) -> Pattern<'tcx> {
        // When implicit dereferences have been inserted in this pattern, the unadjusted lowered
        // pattern has the type that results *after* dereferencing. For example, in this code:
        //
        // ```
        // match &&Some(0i32) {
        //     Some(n) => { ... },
        //     _ => { ... },
        // }
        // ```
        //
        // the type assigned to `Some(n)` in `unadjusted_pat` would be `Option<i32>` (this is
        // determined in rustc_typeck::check::match). The adjustments would be
        //
        // `vec![&&Option<i32>, &Option<i32>]`.
        //
        // Applying the adjustments, we want to instead output `&&Some(n)` (as a HAIR pattern). So
        // we wrap the unadjusted pattern in `PatternKind::Deref` repeatedly, consuming the
        // adjustments in *reverse order* (last-in-first-out, so that the last `Deref` inserted
        // gets the least-dereferenced type).
        let unadjusted_pat = self.lower_pattern_unadjusted(pat);
        self.tables
            .pat_adjustments()
            .get(pat.hir_id)
            .unwrap_or(&vec![])
            .iter()
            .rev()
            .fold(unadjusted_pat, |pat, ref_ty| {
                    debug!("{:?}: wrapping pattern with type {:?}", pat, ref_ty);
                    Pattern {
                        span: pat.span,
                        ty: ref_ty,
                        kind: Box::new(PatternKind::Deref { subpattern: pat }),
                    }
                },
            )
    }

    fn lower_pattern_unadjusted(&mut self, pat: &'tcx hir::Pat) -> Pattern<'tcx> {
        let mut ty = self.tables.node_id_to_type(pat.hir_id);

        let kind = match pat.node {
            PatKind::Wild => PatternKind::Wild,

            PatKind::Lit(ref value) => self.lower_lit(value),

            PatKind::Range(ref lo_expr, ref hi_expr, end) => {
                match (self.lower_lit(lo_expr), self.lower_lit(hi_expr)) {
                    (PatternKind::Constant { value: lo },
                     PatternKind::Constant { value: hi }) => {
                        use std::cmp::Ordering;
                        let cmp = compare_const_vals(
                            self.tcx,
                            lo,
                            hi,
                            self.param_env.and(ty),
                        );
                        match (end, cmp) {
                            (RangeEnd::Excluded, Some(Ordering::Less)) =>
                                PatternKind::Range { lo, hi, end },
                            (RangeEnd::Excluded, _) => {
                                span_err!(
                                    self.tcx.sess,
                                    lo_expr.span,
                                    E0579,
                                    "lower range bound must be less than upper",
                                );
                                PatternKind::Wild
                            },
                            (RangeEnd::Included, None) |
                            (RangeEnd::Included, Some(Ordering::Greater)) => {
                                let mut err = struct_span_err!(
                                    self.tcx.sess,
                                    lo_expr.span,
                                    E0030,
                                    "lower range bound must be less than or equal to upper"
                                );
                                err.span_label(
                                    lo_expr.span,
                                    "lower bound larger than upper bound",
                                );
                                if self.tcx.sess.teach(&err.get_code().unwrap()) {
                                    err.note("When matching against a range, the compiler \
                                              verifies that the range is non-empty. Range \
                                              patterns include both end-points, so this is \
                                              equivalent to requiring the start of the range \
                                              to be less than or equal to the end of the range.");
                                }
                                err.emit();
                                PatternKind::Wild
                            },
                            (RangeEnd::Included, Some(_)) => PatternKind::Range { lo, hi, end },
                        }
                    }
                    _ => PatternKind::Wild
                }
            }

            PatKind::Path(ref qpath) => {
                return self.lower_path(qpath, pat.hir_id, pat.span);
            }

            PatKind::Ref(ref subpattern, _) |
            PatKind::Box(ref subpattern) => {
                PatternKind::Deref { subpattern: self.lower_pattern(subpattern) }
            }

            PatKind::Slice(ref prefix, ref slice, ref suffix) => {
                let ty = self.tables.node_id_to_type(pat.hir_id);
                match ty.sty {
                    ty::TyRef(_, ty, _) =>
                        PatternKind::Deref {
                            subpattern: Pattern {
                                ty,
                                span: pat.span,
                                kind: Box::new(self.slice_or_array_pattern(
                                    pat.span, ty, prefix, slice, suffix))
                            },
                        },

                    ty::TySlice(..) |
                    ty::TyArray(..) =>
                        self.slice_or_array_pattern(pat.span, ty, prefix, slice, suffix),

                    ref sty =>
                        span_bug!(
                            pat.span,
                            "unexpanded type for vector pattern: {:?}",
                            sty),
                }
            }

            PatKind::Tuple(ref subpatterns, ddpos) => {
                let ty = self.tables.node_id_to_type(pat.hir_id);
                match ty.sty {
                    ty::TyTuple(ref tys) => {
                        let subpatterns =
                            subpatterns.iter()
                                       .enumerate_and_adjust(tys.len(), ddpos)
                                       .map(|(i, subpattern)| FieldPattern {
                                            field: Field::new(i),
                                            pattern: self.lower_pattern(subpattern)
                                       })
                                       .collect();

                        PatternKind::Leaf { subpatterns: subpatterns }
                    }

                    ref sty => span_bug!(pat.span, "unexpected type for tuple pattern: {:?}", sty),
                }
            }

            PatKind::Binding(_, id, ref name, ref sub) => {
                let var_ty = self.tables.node_id_to_type(pat.hir_id);
                let region = match var_ty.sty {
                    ty::TyRef(r, _, _) => Some(r),
                    _ => None,
                };
                let bm = *self.tables.pat_binding_modes().get(pat.hir_id)
                                                         .expect("missing binding mode");
                let (mutability, mode) = match bm {
                    ty::BindByValue(hir::MutMutable) =>
                        (Mutability::Mut, BindingMode::ByValue),
                    ty::BindByValue(hir::MutImmutable) =>
                        (Mutability::Not, BindingMode::ByValue),
                    ty::BindByReference(hir::MutMutable) =>
                        (Mutability::Not, BindingMode::ByRef(
                            region.unwrap(), BorrowKind::Mut { allow_two_phase_borrow: false })),
                    ty::BindByReference(hir::MutImmutable) =>
                        (Mutability::Not, BindingMode::ByRef(
                            region.unwrap(), BorrowKind::Shared)),
                };

                // A ref x pattern is the same node used for x, and as such it has
                // x's type, which is &T, where we want T (the type being matched).
                if let ty::BindByReference(_) = bm {
                    if let ty::TyRef(_, rty, _) = ty.sty {
                        ty = rty;
                    } else {
                        bug!("`ref {}` has wrong type {}", name.node, ty);
                    }
                }

                PatternKind::Binding {
                    mutability,
                    mode,
                    name: name.node,
                    var: id,
                    ty: var_ty,
                    subpattern: self.lower_opt_pattern(sub),
                }
            }

            PatKind::TupleStruct(ref qpath, ref subpatterns, ddpos) => {
                let def = self.tables.qpath_def(qpath, pat.hir_id);
                let adt_def = match ty.sty {
                    ty::TyAdt(adt_def, _) => adt_def,
                    _ => span_bug!(pat.span, "tuple struct pattern not applied to an ADT"),
                };
                let variant_def = adt_def.variant_of_def(def);

                let subpatterns =
                        subpatterns.iter()
                                   .enumerate_and_adjust(variant_def.fields.len(), ddpos)
                                   .map(|(i, field)| FieldPattern {
                                       field: Field::new(i),
                                       pattern: self.lower_pattern(field),
                                   })
                                   .collect();
                self.lower_variant_or_leaf(def, pat.span, ty, subpatterns)
            }

            PatKind::Struct(ref qpath, ref fields, _) => {
                let def = self.tables.qpath_def(qpath, pat.hir_id);
                let subpatterns =
                    fields.iter()
                          .map(|field| {
                              FieldPattern {
                                  field: Field::new(self.tcx.field_index(field.node.id,
                                                                         self.tables)),
                                  pattern: self.lower_pattern(&field.node.pat),
                              }
                          })
                          .collect();

                self.lower_variant_or_leaf(def, pat.span, ty, subpatterns)
            }
        };

        Pattern {
            span: pat.span,
            ty,
            kind: Box::new(kind),
        }
    }

    fn lower_patterns(&mut self, pats: &'tcx [P<hir::Pat>]) -> Vec<Pattern<'tcx>> {
        pats.iter().map(|p| self.lower_pattern(p)).collect()
    }

    fn lower_opt_pattern(&mut self, pat: &'tcx Option<P<hir::Pat>>) -> Option<Pattern<'tcx>>
    {
        pat.as_ref().map(|p| self.lower_pattern(p))
    }

    fn flatten_nested_slice_patterns(
        &mut self,
        prefix: Vec<Pattern<'tcx>>,
        slice: Option<Pattern<'tcx>>,
        suffix: Vec<Pattern<'tcx>>)
        -> (Vec<Pattern<'tcx>>, Option<Pattern<'tcx>>, Vec<Pattern<'tcx>>)
    {
        let orig_slice = match slice {
            Some(orig_slice) => orig_slice,
            None => return (prefix, slice, suffix)
        };
        let orig_prefix = prefix;
        let orig_suffix = suffix;

        // dance because of intentional borrow-checker stupidity.
        let kind = *orig_slice.kind;
        match kind {
            PatternKind::Slice { prefix, slice, mut suffix } |
            PatternKind::Array { prefix, slice, mut suffix } => {
                let mut orig_prefix = orig_prefix;

                orig_prefix.extend(prefix);
                suffix.extend(orig_suffix);

                (orig_prefix, slice, suffix)
            }
            _ => {
                (orig_prefix, Some(Pattern {
                    kind: box kind, ..orig_slice
                }), orig_suffix)
            }
        }
    }

    fn slice_or_array_pattern(
        &mut self,
        span: Span,
        ty: Ty<'tcx>,
        prefix: &'tcx [P<hir::Pat>],
        slice: &'tcx Option<P<hir::Pat>>,
        suffix: &'tcx [P<hir::Pat>])
        -> PatternKind<'tcx>
    {
        let prefix = self.lower_patterns(prefix);
        let slice = self.lower_opt_pattern(slice);
        let suffix = self.lower_patterns(suffix);
        let (prefix, slice, suffix) =
            self.flatten_nested_slice_patterns(prefix, slice, suffix);

        match ty.sty {
            ty::TySlice(..) => {
                // matching a slice or fixed-length array
                PatternKind::Slice { prefix: prefix, slice: slice, suffix: suffix }
            }

            ty::TyArray(_, len) => {
                // fixed-length array
                let len = len.unwrap_usize(self.tcx);
                assert!(len >= prefix.len() as u64 + suffix.len() as u64);
                PatternKind::Array { prefix: prefix, slice: slice, suffix: suffix }
            }

            _ => {
                span_bug!(span, "bad slice pattern type {:?}", ty);
            }
        }
    }

    fn lower_variant_or_leaf(
        &mut self,
        def: Def,
        span: Span,
        ty: Ty<'tcx>,
        subpatterns: Vec<FieldPattern<'tcx>>)
        -> PatternKind<'tcx>
    {
        match def {
            Def::Variant(variant_id) | Def::VariantCtor(variant_id, ..) => {
                let enum_id = self.tcx.parent_def_id(variant_id).unwrap();
                let adt_def = self.tcx.adt_def(enum_id);
                if adt_def.is_enum() {
                    let substs = match ty.sty {
                        ty::TyAdt(_, substs) |
                        ty::TyFnDef(_, substs) => substs,
                        _ => bug!("inappropriate type for def: {:?}", ty.sty),
                    };
                    PatternKind::Variant {
                        adt_def,
                        substs,
                        variant_index: adt_def.variant_index_with_id(variant_id),
                        subpatterns,
                    }
                } else {
                    PatternKind::Leaf { subpatterns: subpatterns }
                }
            }

            Def::Struct(..) | Def::StructCtor(..) | Def::Union(..) |
            Def::TyAlias(..) | Def::AssociatedTy(..) | Def::SelfTy(..) => {
                PatternKind::Leaf { subpatterns: subpatterns }
            }

            _ => {
                self.errors.push(PatternError::NonConstPath(span));
                PatternKind::Wild
            }
        }
    }

    /// Takes a HIR Path. If the path is a constant, evaluates it and feeds
    /// it to `const_to_pat`. Any other path (like enum variants without fields)
    /// is converted to the corresponding pattern via `lower_variant_or_leaf`
    fn lower_path(&mut self,
                  qpath: &hir::QPath,
                  id: hir::HirId,
                  span: Span)
                  -> Pattern<'tcx> {
        let ty = self.tables.node_id_to_type(id);
        let def = self.tables.qpath_def(qpath, id);
        let is_associated_const = match def {
            Def::AssociatedConst(_) => true,
            _ => false,
        };
        let kind = match def {
            Def::Const(def_id) | Def::AssociatedConst(def_id) => {
                let substs = self.tables.node_substs(id);
                match ty::Instance::resolve(
                    self.tcx,
                    self.param_env,
                    def_id,
                    substs,
                ) {
                    Some(instance) => {
                        let cid = GlobalId {
                            instance,
                            promoted: None,
                        };
                        match self.tcx.at(span).const_eval(self.param_env.and(cid)) {
                            Ok(value) => {
                                return self.const_to_pat(instance, value, id, span)
                            },
                            Err(err) => {
                                err.report_as_error(
                                    self.tcx.at(span),
                                    "could not evaluate constant pattern",
                                );
                                PatternKind::Wild
                            },
                        }
                    },
                    None => {
                        self.errors.push(if is_associated_const {
                            PatternError::AssociatedConstInPattern(span)
                        } else {
                            PatternError::StaticInPattern(span)
                        });
                        PatternKind::Wild
                    },
                }
            }
            _ => self.lower_variant_or_leaf(def, span, ty, vec![]),
        };

        Pattern {
            span,
            ty,
            kind: Box::new(kind),
        }
    }

    /// Converts literals, paths and negation of literals to patterns.
    /// The special case for negation exists to allow things like -128i8
    /// which would overflow if we tried to evaluate 128i8 and then negate
    /// afterwards.
    fn lower_lit(&mut self, expr: &'tcx hir::Expr) -> PatternKind<'tcx> {
        match expr.node {
            hir::ExprLit(ref lit) => {
                let ty = self.tables.expr_ty(expr);
                match lit_to_const(&lit.node, self.tcx, ty, false) {
                    Ok(val) => {
                        let instance = ty::Instance::new(
                            self.tables.local_id_root.expect("literal outside any scope"),
                            self.substs,
                        );
                        *self.const_to_pat(instance, val, expr.hir_id, lit.span).kind
                    },
                    Err(()) => {
                        self.errors.push(PatternError::FloatBug);
                        PatternKind::Wild
                    },
                }
            },
            hir::ExprPath(ref qpath) => *self.lower_path(qpath, expr.hir_id, expr.span).kind,
            hir::ExprUnary(hir::UnNeg, ref expr) => {
                let ty = self.tables.expr_ty(expr);
                let lit = match expr.node {
                    hir::ExprLit(ref lit) => lit,
                    _ => span_bug!(expr.span, "not a literal: {:?}", expr),
                };
                match lit_to_const(&lit.node, self.tcx, ty, true) {
                    Ok(val) => {
                        let instance = ty::Instance::new(
                            self.tables.local_id_root.expect("literal outside any scope"),
                            self.substs,
                        );
                        *self.const_to_pat(instance, val, expr.hir_id, lit.span).kind
                    },
                    Err(()) => {
                        self.errors.push(PatternError::FloatBug);
                        PatternKind::Wild
                    },
                }
            }
            _ => span_bug!(expr.span, "not a literal: {:?}", expr),
        }
    }

    /// Converts an evaluated constant to a pattern (if possible).
    /// This means aggregate values (like structs and enums) are converted
    /// to a pattern that matches the value (as if you'd compare via eq).
    fn const_to_pat(
        &self,
        instance: ty::Instance<'tcx>,
        cv: &'tcx ty::Const<'tcx>,
        id: hir::HirId,
        span: Span,
    ) -> Pattern<'tcx> {
        debug!("const_to_pat: cv={:#?}", cv);
        let adt_subpattern = |i, variant_opt| {
            let field = Field::new(i);
            let val = match cv.val {
                ConstVal::Value(miri) => const_val_field(
                    self.tcx, self.param_env, instance,
                    variant_opt, field, miri, cv.ty,
                ).expect("field access failed"),
                _ => bug!("{:#?} is not a valid adt", cv),
            };
            self.const_to_pat(instance, val, id, span)
        };
        let adt_subpatterns = |n, variant_opt| {
            (0..n).map(|i| {
                let field = Field::new(i);
                FieldPattern {
                    field,
                    pattern: adt_subpattern(i, variant_opt),
                }
            }).collect::<Vec<_>>()
        };
        let kind = match cv.ty.sty {
            ty::TyFloat(_) => {
                let id = self.tcx.hir.hir_to_node_id(id);
                self.tcx.lint_node(
                    ::rustc::lint::builtin::ILLEGAL_FLOATING_POINT_LITERAL_PATTERN,
                    id,
                    span,
                    "floating-point types cannot be used in patterns",
                );
                PatternKind::Constant {
                    value: cv,
                }
            },
            ty::TyAdt(adt_def, _) if adt_def.is_union() => {
                // Matching on union fields is unsafe, we can't hide it in constants
                self.tcx.sess.span_err(span, "cannot use unions in constant patterns");
                PatternKind::Wild
            }
            ty::TyAdt(adt_def, _) if !self.tcx.has_attr(adt_def.did, "structural_match") => {
                let msg = format!("to use a constant of type `{}` in a pattern, \
                                    `{}` must be annotated with `#[derive(PartialEq, Eq)]`",
                                    self.tcx.item_path_str(adt_def.did),
                                    self.tcx.item_path_str(adt_def.did));
                self.tcx.sess.span_err(span, &msg);
                PatternKind::Wild
            },
            ty::TyAdt(adt_def, substs) if adt_def.is_enum() => {
                match cv.val {
                    ConstVal::Value(val) => {
                        let variant_index = const_variant_index(
                            self.tcx, self.param_env, instance, val, cv.ty
                        ).expect("const_variant_index failed");
                        let subpatterns = adt_subpatterns(
                            adt_def.variants[variant_index].fields.len(),
                            Some(variant_index),
                        );
                        PatternKind::Variant {
                            adt_def,
                            substs,
                            variant_index,
                            subpatterns,
                        }
                    },
                    ConstVal::Unevaluated(..) =>
                        span_bug!(span, "{:#?} is not a valid enum constant", cv),
                }
            },
            ty::TyAdt(adt_def, _) => {
                let struct_var = adt_def.non_enum_variant();
                PatternKind::Leaf {
                    subpatterns: adt_subpatterns(struct_var.fields.len(), None),
                }
            }
            ty::TyTuple(fields) => {
                PatternKind::Leaf {
                    subpatterns: adt_subpatterns(fields.len(), None),
                }
            }
            ty::TyArray(_, n) => {
                PatternKind::Array {
                    prefix: (0..n.unwrap_usize(self.tcx))
                        .map(|i| adt_subpattern(i as usize, None))
                        .collect(),
                    slice: None,
                    suffix: Vec::new(),
                }
            }
            _ => {
                PatternKind::Constant {
                    value: cv,
                }
            },
        };

        Pattern {
            span,
            ty: cv.ty,
            kind: Box::new(kind),
        }
    }
}

pub trait PatternFoldable<'tcx> : Sized {
    fn fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        self.super_fold_with(folder)
    }

    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self;
}

pub trait PatternFolder<'tcx> : Sized {
    fn fold_pattern(&mut self, pattern: &Pattern<'tcx>) -> Pattern<'tcx> {
        pattern.super_fold_with(self)
    }

    fn fold_pattern_kind(&mut self, kind: &PatternKind<'tcx>) -> PatternKind<'tcx> {
        kind.super_fold_with(self)
    }
}


impl<'tcx, T: PatternFoldable<'tcx>> PatternFoldable<'tcx> for Box<T> {
    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        let content: T = (**self).fold_with(folder);
        box content
    }
}

impl<'tcx, T: PatternFoldable<'tcx>> PatternFoldable<'tcx> for Vec<T> {
    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        self.iter().map(|t| t.fold_with(folder)).collect()
    }
}

impl<'tcx, T: PatternFoldable<'tcx>> PatternFoldable<'tcx> for Option<T> {
    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self{
        self.as_ref().map(|t| t.fold_with(folder))
    }
}

macro_rules! CloneImpls {
    (<$lt_tcx:tt> $($ty:ty),+) => {
        $(
            impl<$lt_tcx> PatternFoldable<$lt_tcx> for $ty {
                fn super_fold_with<F: PatternFolder<$lt_tcx>>(&self, _: &mut F) -> Self {
                    Clone::clone(self)
                }
            }
        )+
    }
}

CloneImpls!{ <'tcx>
    Span, Field, Mutability, ast::Name, ast::NodeId, usize, &'tcx ty::Const<'tcx>,
    Region<'tcx>, Ty<'tcx>, BindingMode<'tcx>, &'tcx AdtDef,
    &'tcx Substs<'tcx>, &'tcx Kind<'tcx>
}

impl<'tcx> PatternFoldable<'tcx> for FieldPattern<'tcx> {
    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        FieldPattern {
            field: self.field.fold_with(folder),
            pattern: self.pattern.fold_with(folder)
        }
    }
}

impl<'tcx> PatternFoldable<'tcx> for Pattern<'tcx> {
    fn fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        folder.fold_pattern(self)
    }

    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        Pattern {
            ty: self.ty.fold_with(folder),
            span: self.span.fold_with(folder),
            kind: self.kind.fold_with(folder)
        }
    }
}

impl<'tcx> PatternFoldable<'tcx> for PatternKind<'tcx> {
    fn fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        folder.fold_pattern_kind(self)
    }

    fn super_fold_with<F: PatternFolder<'tcx>>(&self, folder: &mut F) -> Self {
        match *self {
            PatternKind::Wild => PatternKind::Wild,
            PatternKind::Binding {
                mutability,
                name,
                mode,
                var,
                ty,
                ref subpattern,
            } => PatternKind::Binding {
                mutability: mutability.fold_with(folder),
                name: name.fold_with(folder),
                mode: mode.fold_with(folder),
                var: var.fold_with(folder),
                ty: ty.fold_with(folder),
                subpattern: subpattern.fold_with(folder),
            },
            PatternKind::Variant {
                adt_def,
                substs,
                variant_index,
                ref subpatterns,
            } => PatternKind::Variant {
                adt_def: adt_def.fold_with(folder),
                substs: substs.fold_with(folder),
                variant_index: variant_index.fold_with(folder),
                subpatterns: subpatterns.fold_with(folder)
            },
            PatternKind::Leaf {
                ref subpatterns,
            } => PatternKind::Leaf {
                subpatterns: subpatterns.fold_with(folder),
            },
            PatternKind::Deref {
                ref subpattern,
            } => PatternKind::Deref {
                subpattern: subpattern.fold_with(folder),
            },
            PatternKind::Constant {
                value
            } => PatternKind::Constant {
                value: value.fold_with(folder)
            },
            PatternKind::Range {
                lo,
                hi,
                end,
            } => PatternKind::Range {
                lo: lo.fold_with(folder),
                hi: hi.fold_with(folder),
                end,
            },
            PatternKind::Slice {
                ref prefix,
                ref slice,
                ref suffix,
            } => PatternKind::Slice {
                prefix: prefix.fold_with(folder),
                slice: slice.fold_with(folder),
                suffix: suffix.fold_with(folder)
            },
            PatternKind::Array {
                ref prefix,
                ref slice,
                ref suffix
            } => PatternKind::Array {
                prefix: prefix.fold_with(folder),
                slice: slice.fold_with(folder),
                suffix: suffix.fold_with(folder)
            },
        }
    }
}

pub fn compare_const_vals<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    a: &'tcx ty::Const<'tcx>,
    b: &'tcx ty::Const<'tcx>,
    ty: ty::ParamEnvAnd<'tcx, Ty<'tcx>>,
) -> Option<Ordering> {
    trace!("compare_const_vals: {:?}, {:?}", a, b);

    let from_bool = |v: bool| {
        if v {
            Some(Ordering::Equal)
        } else {
            None
        }
    };

    let fallback = || from_bool(a == b);

    // Use the fallback if any type differs
    if a.ty != b.ty || a.ty != ty.value {
        return fallback();
    }

    // FIXME: This should use assert_bits(ty) instead of use_bits
    // but triggers possibly bugs due to mismatching of arrays and slices
    if let (Some(a), Some(b)) = (a.to_bits(tcx, ty), b.to_bits(tcx, ty)) {
        use ::rustc_apfloat::Float;
        return match ty.value.sty {
            ty::TyFloat(ast::FloatTy::F32) => {
                let l = ::rustc_apfloat::ieee::Single::from_bits(a);
                let r = ::rustc_apfloat::ieee::Single::from_bits(b);
                l.partial_cmp(&r)
            },
            ty::TyFloat(ast::FloatTy::F64) => {
                let l = ::rustc_apfloat::ieee::Double::from_bits(a);
                let r = ::rustc_apfloat::ieee::Double::from_bits(b);
                l.partial_cmp(&r)
            },
            ty::TyInt(_) => {
                let a = interpret::sign_extend(tcx, a, ty.value).expect("layout error for TyInt");
                let b = interpret::sign_extend(tcx, b, ty.value).expect("layout error for TyInt");
                Some((a as i128).cmp(&(b as i128)))
            },
            _ => Some(a.cmp(&b)),
        }
    }

    if let ty::TyRef(_, rty, _) = ty.value.sty {
        if let ty::TyStr = rty.sty {
            match (a.to_byval_value(), b.to_byval_value()) {
                (
                    Some(Value::ScalarPair(
                        Scalar::Ptr(ptr_a),
                        len_a,
                    )),
                    Some(Value::ScalarPair(
                        Scalar::Ptr(ptr_b),
                        len_b,
                    ))
                ) if ptr_a.offset.bytes() == 0 && ptr_b.offset.bytes() == 0 => {
                    if let Ok(len_a) = len_a.to_bits(tcx.data_layout.pointer_size) {
                        if let Ok(len_b) = len_b.to_bits(tcx.data_layout.pointer_size) {
                            if len_a == len_b {
                                let map = tcx.alloc_map.lock();
                                let alloc_a = map.unwrap_memory(ptr_a.alloc_id);
                                let alloc_b = map.unwrap_memory(ptr_b.alloc_id);
                                if alloc_a.bytes.len() as u128 == len_a {
                                    return from_bool(alloc_a == alloc_b);
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fallback()
}

// FIXME: Combine with rustc_mir::hair::cx::const_eval_literal
fn lit_to_const<'a, 'tcx>(lit: &'tcx ast::LitKind,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          ty: Ty<'tcx>,
                          neg: bool)
                          -> Result<&'tcx ty::Const<'tcx>, ()> {
    use syntax::ast::*;

    use rustc::mir::interpret::*;
    let lit = match *lit {
        LitKind::Str(ref s, _) => {
            let s = s.as_str();
            let id = tcx.allocate_bytes(s.as_bytes());
            let value = Scalar::Ptr(id.into()).to_value_with_len(s.len() as u64, tcx);
            ConstValue::from_byval_value(value)
        },
        LitKind::ByteStr(ref data) => {
            let id = tcx.allocate_bytes(data);
            ConstValue::Scalar(Scalar::Ptr(id.into()))
        },
        LitKind::Byte(n) => ConstValue::Scalar(Scalar::Bits {
            bits: n as u128,
            defined: 8,
        }),
        LitKind::Int(n, _) => {
            enum Int {
                Signed(IntTy),
                Unsigned(UintTy),
            }
            let ity = match ty.sty {
                ty::TyInt(IntTy::Isize) => Int::Signed(tcx.sess.target.isize_ty),
                ty::TyInt(other) => Int::Signed(other),
                ty::TyUint(UintTy::Usize) => Int::Unsigned(tcx.sess.target.usize_ty),
                ty::TyUint(other) => Int::Unsigned(other),
                _ => bug!(),
            };
            // This converts from LitKind::Int (which is sign extended) to
            // Scalar::Bytes (which is zero extended)
            let n = match ity {
                // FIXME(oli-obk): are these casts correct?
                Int::Signed(IntTy::I8) if neg =>
                    (n as i8).overflowing_neg().0 as u8 as u128,
                Int::Signed(IntTy::I16) if neg =>
                    (n as i16).overflowing_neg().0 as u16 as u128,
                Int::Signed(IntTy::I32) if neg =>
                    (n as i32).overflowing_neg().0 as u32 as u128,
                Int::Signed(IntTy::I64) if neg =>
                    (n as i64).overflowing_neg().0 as u64 as u128,
                Int::Signed(IntTy::I128) if neg =>
                    (n as i128).overflowing_neg().0 as u128,
                Int::Signed(IntTy::I8) | Int::Unsigned(UintTy::U8) => n as u8 as u128,
                Int::Signed(IntTy::I16) | Int::Unsigned(UintTy::U16) => n as u16 as u128,
                Int::Signed(IntTy::I32) | Int::Unsigned(UintTy::U32) => n as u32 as u128,
                Int::Signed(IntTy::I64) | Int::Unsigned(UintTy::U64) => n as u64 as u128,
                Int::Signed(IntTy::I128)| Int::Unsigned(UintTy::U128) => n,
                _ => bug!(),
            };
            let defined = tcx.layout_of(ty::ParamEnv::empty().and(ty)).unwrap().size.bits() as u8;
            ConstValue::Scalar(Scalar::Bits {
                bits: n,
                defined,
            })
        },
        LitKind::Float(n, fty) => {
            parse_float(n, fty, neg)?
        }
        LitKind::FloatUnsuffixed(n) => {
            let fty = match ty.sty {
                ty::TyFloat(fty) => fty,
                _ => bug!()
            };
            parse_float(n, fty, neg)?
        }
        LitKind::Bool(b) => ConstValue::Scalar(Scalar::Bits {
            bits: b as u128,
            defined: 8,
        }),
        LitKind::Char(c) => ConstValue::Scalar(Scalar::Bits {
            bits: c as u128,
            defined: 32,
        }),
    };
    Ok(ty::Const::from_const_value(tcx, lit, ty))
}

pub fn parse_float<'tcx>(
    num: Symbol,
    fty: ast::FloatTy,
    neg: bool,
) -> Result<ConstValue<'tcx>, ()> {
    let num = num.as_str();
    use rustc_apfloat::ieee::{Single, Double};
    use rustc_apfloat::Float;
    let (bits, defined) = match fty {
        ast::FloatTy::F32 => {
            num.parse::<f32>().map_err(|_| ())?;
            let mut f = num.parse::<Single>().unwrap_or_else(|e| {
                panic!("apfloat::ieee::Single failed to parse `{}`: {:?}", num, e)
            });
            if neg {
                f = -f;
            }
            (f.to_bits(), 32)
        }
        ast::FloatTy::F64 => {
            num.parse::<f64>().map_err(|_| ())?;
            let mut f = num.parse::<Double>().unwrap_or_else(|e| {
                panic!("apfloat::ieee::Single failed to parse `{}`: {:?}", num, e)
            });
            if neg {
                f = -f;
            }
            (f.to_bits(), 64)
        }
    };

    Ok(ConstValue::Scalar(Scalar::Bits { bits, defined }))
}
