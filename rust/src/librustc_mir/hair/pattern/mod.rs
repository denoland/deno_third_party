//! Code to validate patterns/matches

mod _match;
mod check_match;

pub use self::check_match::check_crate;
pub(crate) use self::check_match::check_match;

use const_eval::{const_field, const_variant_index};

use hair::util::UserAnnotatedTyHelpers;
use hair::constant::*;

use rustc::mir::{fmt_const_val, Field, BorrowKind, Mutability};
use rustc::mir::{ProjectionElem, UserTypeProjection};
use rustc::mir::interpret::{Scalar, GlobalId, ConstValue, sign_extend};
use rustc::ty::{self, Region, TyCtxt, AdtDef, Ty, Lift};
use rustc::ty::{CanonicalUserTypeAnnotation, CanonicalUserTypeAnnotations, UserTypeAnnotation};
use rustc::ty::subst::{Substs, Kind};
use rustc::ty::layout::VariantIdx;
use rustc::hir::{self, PatKind, RangeEnd};
use rustc::hir::def::{Def, CtorKind};
use rustc::hir::pat_util::EnumerateAndAdjustIterator;

use rustc_data_structures::indexed_vec::Idx;

use std::cmp::Ordering;
use std::fmt;
use syntax::ast;
use syntax::ptr::P;
use syntax_pos::Span;

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
pub struct PatternTypeProjection<'tcx> {
    pub base: CanonicalUserTypeAnnotation<'tcx>,
    pub projs: Vec<ProjectionElem<'tcx, (), ()>>,
}

impl<'tcx> PatternTypeProjection<'tcx> {
    pub(crate) fn from_user_type(user_annotation: CanonicalUserTypeAnnotation<'tcx>) -> Self {
        Self {
            base: user_annotation,
            projs: Vec::new(),
        }
    }

    pub(crate) fn user_ty(
        self,
        annotations: &mut CanonicalUserTypeAnnotations<'tcx>,
        span: Span,
    ) -> UserTypeProjection<'tcx> {
        UserTypeProjection {
            base: annotations.push((span, self.base)),
            projs: self.projs
        }
    }
}

#[derive(Clone, Debug)]
pub enum PatternKind<'tcx> {
    Wild,

    AscribeUserType {
        user_ty: PatternTypeProjection<'tcx>,
        subpattern: Pattern<'tcx>,
        user_ty_span: Span,
    },

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
        variant_index: VariantIdx,
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
        value: ty::Const<'tcx>,
    },

    Range(PatternRange<'tcx>),

    /// matches against a slice, checking the length and extracting elements.
    /// irrefutable when there is a slice pattern and both `prefix` and `suffix` are empty.
    /// e.g., `&[ref xs..]`.
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatternRange<'tcx> {
    pub lo: ty::Const<'tcx>,
    pub hi: ty::Const<'tcx>,
    pub ty: Ty<'tcx>,
    pub end: RangeEnd,
}

impl<'tcx> fmt::Display for Pattern<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.kind {
            PatternKind::Wild => write!(f, "_"),
            PatternKind::AscribeUserType { ref subpattern, .. } =>
                write!(f, "{}: _", subpattern),
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
                    _ => if let ty::Adt(adt, _) = self.ty.sty {
                        if !adt.is_enum() {
                            Some(&adt.variants[VariantIdx::new(0)])
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
                    write!(f, "{}", variant.ident)?;

                    // Only for Adt we can have `S {...}`,
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
                    ty::Adt(def, _) if def.is_box() => write!(f, "box ")?,
                    ty::Ref(_, _, mutbl) => {
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
                fmt_const_val(f, value)
            }
            PatternKind::Range(PatternRange { lo, hi, ty: _, end }) => {
                fmt_const_val(f, lo)?;
                match end {
                    RangeEnd::Included => write!(f, "..=")?,
                    RangeEnd::Excluded => write!(f, "..")?,
                }
                fmt_const_val(f, hi)
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
                                PatternKind::Range(PatternRange { lo, hi, ty, end }),
                            (RangeEnd::Excluded, _) => {
                                span_err!(
                                    self.tcx.sess,
                                    lo_expr.span,
                                    E0579,
                                    "lower range bound must be less than upper",
                                );
                                PatternKind::Wild
                            }
                            (RangeEnd::Included, Some(Ordering::Equal)) => {
                                PatternKind::Constant { value: lo }
                            }
                            (RangeEnd::Included, Some(Ordering::Less)) => {
                                PatternKind::Range(PatternRange { lo, hi, ty, end })
                            }
                            (RangeEnd::Included, _) => {
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
                            }
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
                match ty.sty {
                    ty::Ref(_, ty, _) =>
                        PatternKind::Deref {
                            subpattern: Pattern {
                                ty,
                                span: pat.span,
                                kind: Box::new(self.slice_or_array_pattern(
                                    pat.span, ty, prefix, slice, suffix))
                            },
                        },
                    ty::Slice(..) |
                    ty::Array(..) =>
                        self.slice_or_array_pattern(pat.span, ty, prefix, slice, suffix),
                    ty::Error => { // Avoid ICE
                        return Pattern { span: pat.span, ty, kind: Box::new(PatternKind::Wild) };
                    }
                    ref sty =>
                        span_bug!(
                            pat.span,
                            "unexpanded type for vector pattern: {:?}",
                            sty),
                }
            }

            PatKind::Tuple(ref subpatterns, ddpos) => {
                match ty.sty {
                    ty::Tuple(ref tys) => {
                        let subpatterns =
                            subpatterns.iter()
                                       .enumerate_and_adjust(tys.len(), ddpos)
                                       .map(|(i, subpattern)| FieldPattern {
                                            field: Field::new(i),
                                            pattern: self.lower_pattern(subpattern)
                                       })
                                       .collect();

                        PatternKind::Leaf { subpatterns }
                    }
                    ty::Error => { // Avoid ICE (#50577)
                        return Pattern { span: pat.span, ty, kind: Box::new(PatternKind::Wild) };
                    }
                    ref sty => span_bug!(pat.span, "unexpected type for tuple pattern: {:?}", sty),
                }
            }

            PatKind::Binding(_, id, ident, ref sub) => {
                let var_ty = self.tables.node_id_to_type(pat.hir_id);
                let region = match var_ty.sty {
                    ty::Ref(r, _, _) => Some(r),
                    ty::Error => { // Avoid ICE
                        return Pattern { span: pat.span, ty, kind: Box::new(PatternKind::Wild) };
                    }
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
                    if let ty::Ref(_, rty, _) = ty.sty {
                        ty = rty;
                    } else {
                        bug!("`ref {}` has wrong type {}", ident, ty);
                    }
                }

                PatternKind::Binding {
                    mutability,
                    mode,
                    name: ident.name,
                    var: id,
                    ty: var_ty,
                    subpattern: self.lower_opt_pattern(sub),
                }
            }

            PatKind::TupleStruct(ref qpath, ref subpatterns, ddpos) => {
                let def = self.tables.qpath_def(qpath, pat.hir_id);
                let adt_def = match ty.sty {
                    ty::Adt(adt_def, _) => adt_def,
                    ty::Error => { // Avoid ICE (#50585)
                        return Pattern { span: pat.span, ty, kind: Box::new(PatternKind::Wild) };
                    }
                    _ => span_bug!(pat.span,
                                   "tuple struct pattern not applied to an ADT {:?}",
                                   ty.sty),
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

                self.lower_variant_or_leaf(def, pat.hir_id, pat.span, ty, subpatterns)
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

                self.lower_variant_or_leaf(def, pat.hir_id, pat.span, ty, subpatterns)
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
            ty::Slice(..) => {
                // matching a slice or fixed-length array
                PatternKind::Slice { prefix: prefix, slice: slice, suffix: suffix }
            }

            ty::Array(_, len) => {
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
        hir_id: hir::HirId,
        span: Span,
        ty: Ty<'tcx>,
        subpatterns: Vec<FieldPattern<'tcx>>,
    ) -> PatternKind<'tcx> {
        let mut kind = match def {
            Def::Variant(variant_id) | Def::VariantCtor(variant_id, ..) => {
                let enum_id = self.tcx.parent_def_id(variant_id).unwrap();
                let adt_def = self.tcx.adt_def(enum_id);
                if adt_def.is_enum() {
                    let substs = match ty.sty {
                        ty::Adt(_, substs) |
                        ty::FnDef(_, substs) => substs,
                        ty::Error => {  // Avoid ICE (#50585)
                            return PatternKind::Wild;
                        }
                        _ => bug!("inappropriate type for def: {:?}", ty.sty),
                    };
                    PatternKind::Variant {
                        adt_def,
                        substs,
                        variant_index: adt_def.variant_index_with_id(variant_id),
                        subpatterns,
                    }
                } else {
                    PatternKind::Leaf { subpatterns }
                }
            }

            Def::Struct(..) | Def::StructCtor(..) | Def::Union(..) |
            Def::TyAlias(..) | Def::AssociatedTy(..) | Def::SelfTy(..) | Def::SelfCtor(..) => {
                PatternKind::Leaf { subpatterns }
            }

            _ => {
                self.errors.push(PatternError::NonConstPath(span));
                PatternKind::Wild
            }
        };

        if let Some(user_ty) = self.user_substs_applied_to_ty_of_hir_id(hir_id) {
            debug!("lower_variant_or_leaf: kind={:?} user_ty={:?} span={:?}", kind, user_ty, span);
            kind = PatternKind::AscribeUserType {
                subpattern: Pattern {
                    span,
                    ty,
                    kind: Box::new(kind),
                },
                user_ty: PatternTypeProjection::from_user_type(user_ty),
                user_ty_span: span,
            };
        }

        kind
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
                                let pattern = self.const_to_pat(instance, value, id, span);
                                if !is_associated_const {
                                    return pattern;
                                }

                                let user_provided_types = self.tables().user_provided_types();
                                return if let Some(u_ty) = user_provided_types.get(id) {
                                    let user_ty = PatternTypeProjection::from_user_type(*u_ty);
                                    Pattern {
                                        span,
                                        kind: Box::new(
                                            PatternKind::AscribeUserType {
                                                subpattern: pattern,
                                                user_ty,
                                                user_ty_span: span,
                                            }
                                        ),
                                        ty: value.ty,
                                    }
                                } else {
                                    pattern
                                }
                            },
                            Err(_) => {
                                self.tcx.sess.span_err(
                                    span,
                                    "could not evaluate constant pattern",
                                );
                                PatternKind::Wild
                            }
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
            _ => self.lower_variant_or_leaf(def, id, span, ty, vec![]),
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
            hir::ExprKind::Lit(ref lit) => {
                let ty = self.tables.expr_ty(expr);
                match lit_to_const(&lit.node, self.tcx, ty, false) {
                    Ok(val) => {
                        let instance = ty::Instance::new(
                            self.tables.local_id_root.expect("literal outside any scope"),
                            self.substs,
                        );
                        *self.const_to_pat(instance, val, expr.hir_id, lit.span).kind
                    },
                    Err(LitToConstError::UnparseableFloat) => {
                        self.errors.push(PatternError::FloatBug);
                        PatternKind::Wild
                    },
                    Err(LitToConstError::Reported) => PatternKind::Wild,
                }
            },
            hir::ExprKind::Path(ref qpath) => *self.lower_path(qpath, expr.hir_id, expr.span).kind,
            hir::ExprKind::Unary(hir::UnNeg, ref expr) => {
                let ty = self.tables.expr_ty(expr);
                let lit = match expr.node {
                    hir::ExprKind::Lit(ref lit) => lit,
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
                    Err(LitToConstError::UnparseableFloat) => {
                        self.errors.push(PatternError::FloatBug);
                        PatternKind::Wild
                    },
                    Err(LitToConstError::Reported) => PatternKind::Wild,
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
        cv: ty::Const<'tcx>,
        id: hir::HirId,
        span: Span,
    ) -> Pattern<'tcx> {
        debug!("const_to_pat: cv={:#?} id={:?}", cv, id);
        let adt_subpattern = |i, variant_opt| {
            let field = Field::new(i);
            let val = const_field(
                self.tcx, self.param_env, instance,
                variant_opt, field, cv,
            ).expect("field access failed");
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
        debug!("const_to_pat: cv.ty={:?} span={:?}", cv.ty, span);
        let kind = match cv.ty.sty {
            ty::Float(_) => {
                let id = self.tcx.hir().hir_to_node_id(id);
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
            ty::Adt(adt_def, _) if adt_def.is_union() => {
                // Matching on union fields is unsafe, we can't hide it in constants
                self.tcx.sess.span_err(span, "cannot use unions in constant patterns");
                PatternKind::Wild
            }
            ty::Adt(adt_def, _) if !self.tcx.has_attr(adt_def.did, "structural_match") => {
                let msg = format!("to use a constant of type `{}` in a pattern, \
                                    `{}` must be annotated with `#[derive(PartialEq, Eq)]`",
                                    self.tcx.item_path_str(adt_def.did),
                                    self.tcx.item_path_str(adt_def.did));
                self.tcx.sess.span_err(span, &msg);
                PatternKind::Wild
            },
            ty::Adt(adt_def, substs) if adt_def.is_enum() => {
                let variant_index = const_variant_index(
                    self.tcx, self.param_env, instance, cv
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
            ty::Adt(adt_def, _) => {
                let struct_var = adt_def.non_enum_variant();
                PatternKind::Leaf {
                    subpatterns: adt_subpatterns(struct_var.fields.len(), None),
                }
            }
            ty::Tuple(fields) => {
                PatternKind::Leaf {
                    subpatterns: adt_subpatterns(fields.len(), None),
                }
            }
            ty::Array(_, n) => {
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

impl UserAnnotatedTyHelpers<'tcx, 'tcx> for PatternContext<'_, 'tcx> {
    fn tcx(&self) -> TyCtxt<'_, 'tcx, 'tcx> {
        self.tcx
    }

    fn tables(&self) -> &ty::TypeckTables<'tcx> {
        self.tables
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
    Span, Field, Mutability, ast::Name, ast::NodeId, usize, ty::Const<'tcx>,
    Region<'tcx>, Ty<'tcx>, BindingMode<'tcx>, &'tcx AdtDef,
    &'tcx Substs<'tcx>, &'tcx Kind<'tcx>, UserTypeAnnotation<'tcx>,
    UserTypeProjection<'tcx>, PatternTypeProjection<'tcx>
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
            PatternKind::AscribeUserType {
                ref subpattern,
                ref user_ty,
                user_ty_span,
            } => PatternKind::AscribeUserType {
                subpattern: subpattern.fold_with(folder),
                user_ty: user_ty.fold_with(folder),
                user_ty_span,
            },
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
                variant_index,
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
            PatternKind::Range(PatternRange {
                lo,
                hi,
                ty,
                end,
            }) => PatternKind::Range(PatternRange {
                lo: lo.fold_with(folder),
                hi: hi.fold_with(folder),
                ty: ty.fold_with(folder),
                end,
            }),
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

pub fn compare_const_vals<'a, 'gcx, 'tcx>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    a: ty::Const<'tcx>,
    b: ty::Const<'tcx>,
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

    let tcx = tcx.global_tcx();
    let (a, b, ty) = (a, b, ty).lift_to_tcx(tcx).unwrap();

    // FIXME: This should use assert_bits(ty) instead of use_bits
    // but triggers possibly bugs due to mismatching of arrays and slices
    if let (Some(a), Some(b)) = (a.to_bits(tcx, ty), b.to_bits(tcx, ty)) {
        use ::rustc_apfloat::Float;
        return match ty.value.sty {
            ty::Float(ast::FloatTy::F32) => {
                let l = ::rustc_apfloat::ieee::Single::from_bits(a);
                let r = ::rustc_apfloat::ieee::Single::from_bits(b);
                l.partial_cmp(&r)
            },
            ty::Float(ast::FloatTy::F64) => {
                let l = ::rustc_apfloat::ieee::Double::from_bits(a);
                let r = ::rustc_apfloat::ieee::Double::from_bits(b);
                l.partial_cmp(&r)
            },
            ty::Int(_) => {
                let layout = tcx.layout_of(ty).ok()?;
                assert!(layout.abi.is_signed());
                let a = sign_extend(a, layout.size);
                let b = sign_extend(b, layout.size);
                Some((a as i128).cmp(&(b as i128)))
            },
            _ => Some(a.cmp(&b)),
        }
    }

    if let ty::Str = ty.value.sty {
        match (a.val, b.val) {
            (
                ConstValue::ScalarPair(
                    Scalar::Ptr(ptr_a),
                    len_a,
                ),
                ConstValue::ScalarPair(
                    Scalar::Ptr(ptr_b),
                    len_b,
                ),
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

    fallback()
}
