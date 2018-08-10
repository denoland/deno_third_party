// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! misc. type-system utilities too small to deserve their own file

use hir::def::Def;
use hir::def_id::DefId;
use hir::map::{DefPathData, Node};
use hir;
use ich::NodeIdHashingMode;
use traits::{self, ObligationCause};
use ty::{self, Ty, TyCtxt, GenericParamDefKind, TypeFoldable};
use ty::subst::{Substs, UnpackedKind};
use ty::query::TyCtxtAt;
use ty::TypeVariants::*;
use ty::layout::{Integer, IntegerExt};
use util::common::ErrorReported;
use middle::lang_items;

use rustc_data_structures::stable_hasher::{StableHasher, HashStable};
use rustc_data_structures::fx::FxHashMap;
use std::{cmp, fmt};
use syntax::ast;
use syntax::attr::{self, SignedInt, UnsignedInt};
use syntax_pos::{Span, DUMMY_SP};

#[derive(Copy, Clone, Debug)]
pub struct Discr<'tcx> {
    /// bit representation of the discriminant, so `-128i8` is `0xFF_u128`
    pub val: u128,
    pub ty: Ty<'tcx>
}

impl<'tcx> fmt::Display for Discr<'tcx> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.ty.sty {
            ty::TyInt(ity) => {
                let bits = ty::tls::with(|tcx| {
                    Integer::from_attr(tcx, SignedInt(ity)).size().bits()
                });
                let x = self.val as i128;
                // sign extend the raw representation to be an i128
                let x = (x << (128 - bits)) >> (128 - bits);
                write!(fmt, "{}", x)
            },
            _ => write!(fmt, "{}", self.val),
        }
    }
}

impl<'tcx> Discr<'tcx> {
    /// Adds 1 to the value and wraps around if the maximum for the type is reached
    pub fn wrap_incr<'a, 'gcx>(self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Self {
        self.checked_add(tcx, 1).0
    }
    pub fn checked_add<'a, 'gcx>(self, tcx: TyCtxt<'a, 'gcx, 'tcx>, n: u128) -> (Self, bool) {
        let (int, signed) = match self.ty.sty {
            TyInt(ity) => (Integer::from_attr(tcx, SignedInt(ity)), true),
            TyUint(uty) => (Integer::from_attr(tcx, UnsignedInt(uty)), false),
            _ => bug!("non integer discriminant"),
        };

        let bit_size = int.size().bits();
        let shift = 128 - bit_size;
        if signed {
            let sext = |u| {
                let i = u as i128;
                (i << shift) >> shift
            };
            let min = sext(1_u128 << (bit_size - 1));
            let max = i128::max_value() >> shift;
            let val = sext(self.val);
            assert!(n < (i128::max_value() as u128));
            let n = n as i128;
            let oflo = val > max - n;
            let val = if oflo {
                min + (n - (max - val) - 1)
            } else {
                val + n
            };
            // zero the upper bits
            let val = val as u128;
            let val = (val << shift) >> shift;
            (Self {
                val: val as u128,
                ty: self.ty,
            }, oflo)
        } else {
            let max = u128::max_value() >> shift;
            let val = self.val;
            let oflo = val > max - n;
            let val = if oflo {
                n - (max - val) - 1
            } else {
                val + n
            };
            (Self {
                val: val,
                ty: self.ty,
            }, oflo)
        }
    }
}

pub trait IntTypeExt {
    fn to_ty<'a, 'gcx, 'tcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx>;
    fn disr_incr<'a, 'tcx>(&self, tcx: TyCtxt<'a, 'tcx, 'tcx>, val: Option<Discr<'tcx>>)
                           -> Option<Discr<'tcx>>;
    fn initial_discriminant<'a, 'tcx>(&self, tcx: TyCtxt<'a, 'tcx, 'tcx>) -> Discr<'tcx>;
}

impl IntTypeExt for attr::IntType {
    fn to_ty<'a, 'gcx, 'tcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx> {
        match *self {
            SignedInt(ast::IntTy::I8)      => tcx.types.i8,
            SignedInt(ast::IntTy::I16)     => tcx.types.i16,
            SignedInt(ast::IntTy::I32)     => tcx.types.i32,
            SignedInt(ast::IntTy::I64)     => tcx.types.i64,
            SignedInt(ast::IntTy::I128)     => tcx.types.i128,
            SignedInt(ast::IntTy::Isize)   => tcx.types.isize,
            UnsignedInt(ast::UintTy::U8)    => tcx.types.u8,
            UnsignedInt(ast::UintTy::U16)   => tcx.types.u16,
            UnsignedInt(ast::UintTy::U32)   => tcx.types.u32,
            UnsignedInt(ast::UintTy::U64)   => tcx.types.u64,
            UnsignedInt(ast::UintTy::U128)   => tcx.types.u128,
            UnsignedInt(ast::UintTy::Usize) => tcx.types.usize,
        }
    }

    fn initial_discriminant<'a, 'tcx>(&self, tcx: TyCtxt<'a, 'tcx, 'tcx>) -> Discr<'tcx> {
        Discr {
            val: 0,
            ty: self.to_ty(tcx)
        }
    }

    fn disr_incr<'a, 'tcx>(
        &self,
        tcx: TyCtxt<'a, 'tcx, 'tcx>,
        val: Option<Discr<'tcx>>,
    ) -> Option<Discr<'tcx>> {
        if let Some(val) = val {
            assert_eq!(self.to_ty(tcx), val.ty);
            let (new, oflo) = val.checked_add(tcx, 1);
            if oflo {
                None
            } else {
                Some(new)
            }
        } else {
            Some(self.initial_discriminant(tcx))
        }
    }
}


#[derive(Clone)]
pub enum CopyImplementationError<'tcx> {
    InfrigingFields(Vec<&'tcx ty::FieldDef>),
    NotAnAdt,
    HasDestructor,
}

/// Describes whether a type is representable. For types that are not
/// representable, 'SelfRecursive' and 'ContainsRecursive' are used to
/// distinguish between types that are recursive with themselves and types that
/// contain a different recursive type. These cases can therefore be treated
/// differently when reporting errors.
///
/// The ordering of the cases is significant. They are sorted so that cmp::max
/// will keep the "more erroneous" of two values.
#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Debug)]
pub enum Representability {
    Representable,
    ContainsRecursive,
    SelfRecursive(Vec<Span>),
}

impl<'tcx> ty::ParamEnv<'tcx> {
    pub fn can_type_implement_copy<'a>(self,
                                       tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                       self_type: Ty<'tcx>)
                                       -> Result<(), CopyImplementationError<'tcx>> {
        // FIXME: (@jroesch) float this code up
        tcx.infer_ctxt().enter(|infcx| {
            let (adt, substs) = match self_type.sty {
                // These types used to have a builtin impl.
                // Now libcore provides that impl.
                ty::TyUint(_) | ty::TyInt(_) | ty::TyBool | ty::TyFloat(_) |
                ty::TyChar | ty::TyRawPtr(..) | ty::TyNever |
                ty::TyRef(_, _, hir::MutImmutable) => return Ok(()),

                ty::TyAdt(adt, substs) => (adt, substs),

                _ => return Err(CopyImplementationError::NotAnAdt),
            };

            let mut infringing = Vec::new();
            for variant in &adt.variants {
                for field in &variant.fields {
                    let span = tcx.def_span(field.did);
                    let ty = field.ty(tcx, substs);
                    if ty.references_error() {
                        continue;
                    }
                    let cause = ObligationCause { span, ..ObligationCause::dummy() };
                    let ctx = traits::FulfillmentContext::new();
                    match traits::fully_normalize(&infcx, ctx, cause, self, &ty) {
                        Ok(ty) => if infcx.type_moves_by_default(self, ty, span) {
                            infringing.push(field);
                        }
                        Err(errors) => {
                            infcx.report_fulfillment_errors(&errors, None, false);
                        }
                    };
                }
            }
            if !infringing.is_empty() {
                return Err(CopyImplementationError::InfrigingFields(infringing));
            }
            if adt.has_dtor(tcx) {
                return Err(CopyImplementationError::HasDestructor);
            }

            Ok(())
        })
    }
}

impl<'a, 'tcx> TyCtxt<'a, 'tcx, 'tcx> {
    /// Creates a hash of the type `Ty` which will be the same no matter what crate
    /// context it's calculated within. This is used by the `type_id` intrinsic.
    pub fn type_id_hash(self, ty: Ty<'tcx>) -> u64 {
        let mut hasher = StableHasher::new();
        let mut hcx = self.create_stable_hashing_context();

        // We want the type_id be independent of the types free regions, so we
        // erase them. The erase_regions() call will also anonymize bound
        // regions, which is desirable too.
        let ty = self.erase_regions(&ty);

        hcx.while_hashing_spans(false, |hcx| {
            hcx.with_node_id_hashing_mode(NodeIdHashingMode::HashDefPath, |hcx| {
                ty.hash_stable(hcx, &mut hasher);
            });
        });
        hasher.finish()
    }
}

impl<'a, 'gcx, 'tcx> TyCtxt<'a, 'gcx, 'tcx> {
    pub fn has_error_field(self, ty: Ty<'tcx>) -> bool {
        match ty.sty {
            ty::TyAdt(def, substs) => {
                for field in def.all_fields() {
                    let field_ty = field.ty(self, substs);
                    if let TyError = field_ty.sty {
                        return true;
                    }
                }
            }
            _ => (),
        }
        false
    }

    /// Returns the deeply last field of nested structures, or the same type,
    /// if not a structure at all. Corresponds to the only possible unsized
    /// field, and its type can be used to determine unsizing strategy.
    pub fn struct_tail(self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        loop {
            match ty.sty {
                ty::TyAdt(def, substs) => {
                    if !def.is_struct() {
                        break;
                    }
                    match def.non_enum_variant().fields.last() {
                        Some(f) => ty = f.ty(self, substs),
                        None => break,
                    }
                }

                ty::TyTuple(tys) => {
                    if let Some((&last_ty, _)) = tys.split_last() {
                        ty = last_ty;
                    } else {
                        break;
                    }
                }

                _ => {
                    break;
                }
            }
        }
        ty
    }

    /// Same as applying struct_tail on `source` and `target`, but only
    /// keeps going as long as the two types are instances of the same
    /// structure definitions.
    /// For `(Foo<Foo<T>>, Foo<Trait>)`, the result will be `(Foo<T>, Trait)`,
    /// whereas struct_tail produces `T`, and `Trait`, respectively.
    pub fn struct_lockstep_tails(self,
                                 source: Ty<'tcx>,
                                 target: Ty<'tcx>)
                                 -> (Ty<'tcx>, Ty<'tcx>) {
        let (mut a, mut b) = (source, target);
        loop {
            match (&a.sty, &b.sty) {
                (&TyAdt(a_def, a_substs), &TyAdt(b_def, b_substs))
                        if a_def == b_def && a_def.is_struct() => {
                    if let Some(f) = a_def.non_enum_variant().fields.last() {
                        a = f.ty(self, a_substs);
                        b = f.ty(self, b_substs);
                    } else {
                        break;
                    }
                },
                (&TyTuple(a_tys), &TyTuple(b_tys))
                        if a_tys.len() == b_tys.len() => {
                    if let Some(a_last) = a_tys.last() {
                        a = a_last;
                        b = b_tys.last().unwrap();
                    } else {
                        break;
                    }
                },
                _ => break,
            }
        }
        (a, b)
    }

    /// Given a set of predicates that apply to an object type, returns
    /// the region bounds that the (erased) `Self` type must
    /// outlive. Precisely *because* the `Self` type is erased, the
    /// parameter `erased_self_ty` must be supplied to indicate what type
    /// has been used to represent `Self` in the predicates
    /// themselves. This should really be a unique type; `FreshTy(0)` is a
    /// popular choice.
    ///
    /// NB: in some cases, particularly around higher-ranked bounds,
    /// this function returns a kind of conservative approximation.
    /// That is, all regions returned by this function are definitely
    /// required, but there may be other region bounds that are not
    /// returned, as well as requirements like `for<'a> T: 'a`.
    ///
    /// Requires that trait definitions have been processed so that we can
    /// elaborate predicates and walk supertraits.
    ///
    /// FIXME callers may only have a &[Predicate], not a Vec, so that's
    /// what this code should accept.
    pub fn required_region_bounds(self,
                                  erased_self_ty: Ty<'tcx>,
                                  predicates: Vec<ty::Predicate<'tcx>>)
                                  -> Vec<ty::Region<'tcx>>    {
        debug!("required_region_bounds(erased_self_ty={:?}, predicates={:?})",
               erased_self_ty,
               predicates);

        assert!(!erased_self_ty.has_escaping_regions());

        traits::elaborate_predicates(self, predicates)
            .filter_map(|predicate| {
                match predicate {
                    ty::Predicate::Projection(..) |
                    ty::Predicate::Trait(..) |
                    ty::Predicate::Subtype(..) |
                    ty::Predicate::WellFormed(..) |
                    ty::Predicate::ObjectSafe(..) |
                    ty::Predicate::ClosureKind(..) |
                    ty::Predicate::RegionOutlives(..) |
                    ty::Predicate::ConstEvaluatable(..) => {
                        None
                    }
                    ty::Predicate::TypeOutlives(predicate) => {
                        // Search for a bound of the form `erased_self_ty
                        // : 'a`, but be wary of something like `for<'a>
                        // erased_self_ty : 'a` (we interpret a
                        // higher-ranked bound like that as 'static,
                        // though at present the code in `fulfill.rs`
                        // considers such bounds to be unsatisfiable, so
                        // it's kind of a moot point since you could never
                        // construct such an object, but this seems
                        // correct even if that code changes).
                        let ty::OutlivesPredicate(ref t, ref r) = predicate.skip_binder();
                        if t == &erased_self_ty && !r.has_escaping_regions() {
                            Some(*r)
                        } else {
                            None
                        }
                    }
                }
            })
            .collect()
    }

    /// Calculate the destructor of a given type.
    pub fn calculate_dtor(
        self,
        adt_did: DefId,
        validate: &mut dyn FnMut(Self, DefId) -> Result<(), ErrorReported>
    ) -> Option<ty::Destructor> {
        let drop_trait = if let Some(def_id) = self.lang_items().drop_trait() {
            def_id
        } else {
            return None;
        };

        ty::query::queries::coherent_trait::ensure(self, drop_trait);

        let mut dtor_did = None;
        let ty = self.type_of(adt_did);
        self.for_each_relevant_impl(drop_trait, ty, |impl_did| {
            if let Some(item) = self.associated_items(impl_did).next() {
                if let Ok(()) = validate(self, impl_did) {
                    dtor_did = Some(item.def_id);
                }
            }
        });

        Some(ty::Destructor { did: dtor_did? })
    }

    /// Return the set of types that are required to be alive in
    /// order to run the destructor of `def` (see RFCs 769 and
    /// 1238).
    ///
    /// Note that this returns only the constraints for the
    /// destructor of `def` itself. For the destructors of the
    /// contents, you need `adt_dtorck_constraint`.
    pub fn destructor_constraints(self, def: &'tcx ty::AdtDef)
                                  -> Vec<ty::subst::Kind<'tcx>>
    {
        let dtor = match def.destructor(self) {
            None => {
                debug!("destructor_constraints({:?}) - no dtor", def.did);
                return vec![]
            }
            Some(dtor) => dtor.did
        };

        // RFC 1238: if the destructor method is tagged with the
        // attribute `unsafe_destructor_blind_to_params`, then the
        // compiler is being instructed to *assume* that the
        // destructor will not access borrowed data,
        // even if such data is otherwise reachable.
        //
        // Such access can be in plain sight (e.g. dereferencing
        // `*foo.0` of `Foo<'a>(&'a u32)`) or indirectly hidden
        // (e.g. calling `foo.0.clone()` of `Foo<T:Clone>`).
        if self.has_attr(dtor, "unsafe_destructor_blind_to_params") {
            debug!("destructor_constraint({:?}) - blind", def.did);
            return vec![];
        }

        let impl_def_id = self.associated_item(dtor).container.id();
        let impl_generics = self.generics_of(impl_def_id);

        // We have a destructor - all the parameters that are not
        // pure_wrt_drop (i.e, don't have a #[may_dangle] attribute)
        // must be live.

        // We need to return the list of parameters from the ADTs
        // generics/substs that correspond to impure parameters on the
        // impl's generics. This is a bit ugly, but conceptually simple:
        //
        // Suppose our ADT looks like the following
        //
        //     struct S<X, Y, Z>(X, Y, Z);
        //
        // and the impl is
        //
        //     impl<#[may_dangle] P0, P1, P2> Drop for S<P1, P2, P0>
        //
        // We want to return the parameters (X, Y). For that, we match
        // up the item-substs <X, Y, Z> with the substs on the impl ADT,
        // <P1, P2, P0>, and then look up which of the impl substs refer to
        // parameters marked as pure.

        let impl_substs = match self.type_of(impl_def_id).sty {
            ty::TyAdt(def_, substs) if def_ == def => substs,
            _ => bug!()
        };

        let item_substs = match self.type_of(def.did).sty {
            ty::TyAdt(def_, substs) if def_ == def => substs,
            _ => bug!()
        };

        let result = item_substs.iter().zip(impl_substs.iter())
            .filter(|&(_, &k)| {
                match k.unpack() {
                    UnpackedKind::Lifetime(&ty::RegionKind::ReEarlyBound(ref ebr)) => {
                        !impl_generics.region_param(ebr, self).pure_wrt_drop
                    }
                    UnpackedKind::Type(&ty::TyS {
                        sty: ty::TypeVariants::TyParam(ref pt), ..
                    }) => {
                        !impl_generics.type_param(pt, self).pure_wrt_drop
                    }
                    UnpackedKind::Lifetime(_) | UnpackedKind::Type(_) => {
                        // not a type or region param - this should be reported
                        // as an error.
                        false
                    }
                }
            }).map(|(&item_param, _)| item_param).collect();
        debug!("destructor_constraint({:?}) = {:?}", def.did, result);
        result
    }

    pub fn is_closure(self, def_id: DefId) -> bool {
        self.def_key(def_id).disambiguated_data.data == DefPathData::ClosureExpr
    }

    /// Given the `DefId` of a fn or closure, returns the `DefId` of
    /// the innermost fn item that the closure is contained within.
    /// This is a significant def-id because, when we do
    /// type-checking, we type-check this fn item and all of its
    /// (transitive) closures together.  Therefore, when we fetch the
    /// `typeck_tables_of` the closure, for example, we really wind up
    /// fetching the `typeck_tables_of` the enclosing fn item.
    pub fn closure_base_def_id(self, def_id: DefId) -> DefId {
        let mut def_id = def_id;
        while self.is_closure(def_id) {
            def_id = self.parent_def_id(def_id).unwrap_or_else(|| {
                bug!("closure {:?} has no parent", def_id);
            });
        }
        def_id
    }

    /// Given the def-id and substs a closure, creates the type of
    /// `self` argument that the closure expects. For example, for a
    /// `Fn` closure, this would return a reference type `&T` where
    /// `T=closure_ty`.
    ///
    /// Returns `None` if this closure's kind has not yet been inferred.
    /// This should only be possible during type checking.
    ///
    /// Note that the return value is a late-bound region and hence
    /// wrapped in a binder.
    pub fn closure_env_ty(self,
                          closure_def_id: DefId,
                          closure_substs: ty::ClosureSubsts<'tcx>)
                          -> Option<ty::Binder<Ty<'tcx>>>
    {
        let closure_ty = self.mk_closure(closure_def_id, closure_substs);
        let env_region = ty::ReLateBound(ty::INNERMOST, ty::BrEnv);
        let closure_kind_ty = closure_substs.closure_kind_ty(closure_def_id, self);
        let closure_kind = closure_kind_ty.to_opt_closure_kind()?;
        let env_ty = match closure_kind {
            ty::ClosureKind::Fn => self.mk_imm_ref(self.mk_region(env_region), closure_ty),
            ty::ClosureKind::FnMut => self.mk_mut_ref(self.mk_region(env_region), closure_ty),
            ty::ClosureKind::FnOnce => closure_ty,
        };
        Some(ty::Binder::bind(env_ty))
    }

    /// Given the def-id of some item that has no type parameters, make
    /// a suitable "empty substs" for it.
    pub fn empty_substs_for_def_id(self, item_def_id: DefId) -> &'tcx Substs<'tcx> {
        Substs::for_item(self, item_def_id, |param, _| {
            match param.kind {
                GenericParamDefKind::Lifetime => self.types.re_erased.into(),
                GenericParamDefKind::Type {..} => {
                    bug!("empty_substs_for_def_id: {:?} has type parameters", item_def_id)
                }
            }
        })
    }

    /// Return whether the node pointed to by def_id is a static item, and its mutability
    pub fn is_static(&self, def_id: DefId) -> Option<hir::Mutability> {
        if let Some(node) = self.hir.get_if_local(def_id) {
            match node {
                Node::NodeItem(&hir::Item {
                    node: hir::ItemStatic(_, mutbl, _), ..
                }) => Some(mutbl),
                Node::NodeForeignItem(&hir::ForeignItem {
                    node: hir::ForeignItemStatic(_, is_mutbl), ..
                }) =>
                    Some(if is_mutbl {
                        hir::Mutability::MutMutable
                    } else {
                        hir::Mutability::MutImmutable
                    }),
                _ => None
            }
        } else {
            match self.describe_def(def_id) {
                Some(Def::Static(_, is_mutbl)) =>
                    Some(if is_mutbl {
                        hir::Mutability::MutMutable
                    } else {
                        hir::Mutability::MutImmutable
                    }),
                _ => None
            }
        }
    }
}

impl<'a, 'tcx> ty::TyS<'tcx> {
    pub fn moves_by_default(&'tcx self,
                            tcx: TyCtxt<'a, 'tcx, 'tcx>,
                            param_env: ty::ParamEnv<'tcx>,
                            span: Span)
                            -> bool {
        !tcx.at(span).is_copy_raw(param_env.and(self))
    }

    pub fn is_sized(&'tcx self,
                    tcx_at: TyCtxtAt<'a, 'tcx, 'tcx>,
                    param_env: ty::ParamEnv<'tcx>)-> bool
    {
        tcx_at.is_sized_raw(param_env.and(self))
    }

    pub fn is_freeze(&'tcx self,
                     tcx: TyCtxt<'a, 'tcx, 'tcx>,
                     param_env: ty::ParamEnv<'tcx>,
                     span: Span)-> bool
    {
        tcx.at(span).is_freeze_raw(param_env.and(self))
    }

    /// If `ty.needs_drop(...)` returns `true`, then `ty` is definitely
    /// non-copy and *might* have a destructor attached; if it returns
    /// `false`, then `ty` definitely has no destructor (i.e. no drop glue).
    ///
    /// (Note that this implies that if `ty` has a destructor attached,
    /// then `needs_drop` will definitely return `true` for `ty`.)
    #[inline]
    pub fn needs_drop(&'tcx self,
                      tcx: TyCtxt<'a, 'tcx, 'tcx>,
                      param_env: ty::ParamEnv<'tcx>)
                      -> bool {
        tcx.needs_drop_raw(param_env.and(self))
    }

    /// Check whether a type is representable. This means it cannot contain unboxed
    /// structural recursion. This check is needed for structs and enums.
    pub fn is_representable(&'tcx self,
                            tcx: TyCtxt<'a, 'tcx, 'tcx>,
                            sp: Span)
                            -> Representability {

        // Iterate until something non-representable is found
        fn fold_repr<It: Iterator<Item=Representability>>(iter: It) -> Representability {
            iter.fold(Representability::Representable, |r1, r2| {
                match (r1, r2) {
                    (Representability::SelfRecursive(v1),
                     Representability::SelfRecursive(v2)) => {
                        Representability::SelfRecursive(v1.iter().map(|s| *s).chain(v2).collect())
                    }
                    (r1, r2) => cmp::max(r1, r2)
                }
            })
        }

        fn are_inner_types_recursive<'a, 'tcx>(
            tcx: TyCtxt<'a, 'tcx, 'tcx>, sp: Span,
            seen: &mut Vec<Ty<'tcx>>,
            representable_cache: &mut FxHashMap<Ty<'tcx>, Representability>,
            ty: Ty<'tcx>)
            -> Representability
        {
            match ty.sty {
                TyTuple(ref ts) => {
                    // Find non representable
                    fold_repr(ts.iter().map(|ty| {
                        is_type_structurally_recursive(tcx, sp, seen, representable_cache, ty)
                    }))
                }
                // Fixed-length vectors.
                // FIXME(#11924) Behavior undecided for zero-length vectors.
                TyArray(ty, _) => {
                    is_type_structurally_recursive(tcx, sp, seen, representable_cache, ty)
                }
                TyAdt(def, substs) => {
                    // Find non representable fields with their spans
                    fold_repr(def.all_fields().map(|field| {
                        let ty = field.ty(tcx, substs);
                        let span = tcx.hir.span_if_local(field.did).unwrap_or(sp);
                        match is_type_structurally_recursive(tcx, span, seen,
                                                             representable_cache, ty)
                        {
                            Representability::SelfRecursive(_) => {
                                Representability::SelfRecursive(vec![span])
                            }
                            x => x,
                        }
                    }))
                }
                TyClosure(..) => {
                    // this check is run on type definitions, so we don't expect
                    // to see closure types
                    bug!("requires check invoked on inapplicable type: {:?}", ty)
                }
                _ => Representability::Representable,
            }
        }

        fn same_struct_or_enum<'tcx>(ty: Ty<'tcx>, def: &'tcx ty::AdtDef) -> bool {
            match ty.sty {
                TyAdt(ty_def, _) => {
                     ty_def == def
                }
                _ => false
            }
        }

        fn same_type<'tcx>(a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
            match (&a.sty, &b.sty) {
                (&TyAdt(did_a, substs_a), &TyAdt(did_b, substs_b)) => {
                    if did_a != did_b {
                        return false;
                    }

                    substs_a.types().zip(substs_b.types()).all(|(a, b)| same_type(a, b))
                }
                _ => a == b,
            }
        }

        // Does the type `ty` directly (without indirection through a pointer)
        // contain any types on stack `seen`?
        fn is_type_structurally_recursive<'a, 'tcx>(
            tcx: TyCtxt<'a, 'tcx, 'tcx>,
            sp: Span,
            seen: &mut Vec<Ty<'tcx>>,
            representable_cache: &mut FxHashMap<Ty<'tcx>, Representability>,
            ty: Ty<'tcx>) -> Representability
        {
            debug!("is_type_structurally_recursive: {:?} {:?}", ty, sp);
            if let Some(representability) = representable_cache.get(ty) {
                debug!("is_type_structurally_recursive: {:?} {:?} - (cached) {:?}",
                       ty, sp, representability);
                return representability.clone();
            }

            let representability = is_type_structurally_recursive_inner(
                tcx, sp, seen, representable_cache, ty);

            representable_cache.insert(ty, representability.clone());
            representability
        }

        fn is_type_structurally_recursive_inner<'a, 'tcx>(
            tcx: TyCtxt<'a, 'tcx, 'tcx>,
            sp: Span,
            seen: &mut Vec<Ty<'tcx>>,
            representable_cache: &mut FxHashMap<Ty<'tcx>, Representability>,
            ty: Ty<'tcx>) -> Representability
        {
            match ty.sty {
                TyAdt(def, _) => {
                    {
                        // Iterate through stack of previously seen types.
                        let mut iter = seen.iter();

                        // The first item in `seen` is the type we are actually curious about.
                        // We want to return SelfRecursive if this type contains itself.
                        // It is important that we DON'T take generic parameters into account
                        // for this check, so that Bar<T> in this example counts as SelfRecursive:
                        //
                        // struct Foo;
                        // struct Bar<T> { x: Bar<Foo> }

                        if let Some(&seen_type) = iter.next() {
                            if same_struct_or_enum(seen_type, def) {
                                debug!("SelfRecursive: {:?} contains {:?}",
                                       seen_type,
                                       ty);
                                return Representability::SelfRecursive(vec![sp]);
                            }
                        }

                        // We also need to know whether the first item contains other types
                        // that are structurally recursive. If we don't catch this case, we
                        // will recurse infinitely for some inputs.
                        //
                        // It is important that we DO take generic parameters into account
                        // here, so that code like this is considered SelfRecursive, not
                        // ContainsRecursive:
                        //
                        // struct Foo { Option<Option<Foo>> }

                        for &seen_type in iter {
                            if same_type(ty, seen_type) {
                                debug!("ContainsRecursive: {:?} contains {:?}",
                                       seen_type,
                                       ty);
                                return Representability::ContainsRecursive;
                            }
                        }
                    }

                    // For structs and enums, track all previously seen types by pushing them
                    // onto the 'seen' stack.
                    seen.push(ty);
                    let out = are_inner_types_recursive(tcx, sp, seen, representable_cache, ty);
                    seen.pop();
                    out
                }
                _ => {
                    // No need to push in other cases.
                    are_inner_types_recursive(tcx, sp, seen, representable_cache, ty)
                }
            }
        }

        debug!("is_type_representable: {:?}", self);

        // To avoid a stack overflow when checking an enum variant or struct that
        // contains a different, structurally recursive type, maintain a stack
        // of seen types and check recursion for each of them (issues #3008, #3779).
        let mut seen: Vec<Ty> = Vec::new();
        let mut representable_cache = FxHashMap();
        let r = is_type_structurally_recursive(
            tcx, sp, &mut seen, &mut representable_cache, self);
        debug!("is_type_representable: {:?} is {:?}", self, r);
        r
    }
}

fn is_copy_raw<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                         query: ty::ParamEnvAnd<'tcx, Ty<'tcx>>)
                         -> bool
{
    let (param_env, ty) = query.into_parts();
    let trait_def_id = tcx.require_lang_item(lang_items::CopyTraitLangItem);
    tcx.infer_ctxt()
       .enter(|infcx| traits::type_known_to_meet_bound(&infcx,
                                                       param_env,
                                                       ty,
                                                       trait_def_id,
                                                       DUMMY_SP))
}

fn is_sized_raw<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          query: ty::ParamEnvAnd<'tcx, Ty<'tcx>>)
                          -> bool
{
    let (param_env, ty) = query.into_parts();
    let trait_def_id = tcx.require_lang_item(lang_items::SizedTraitLangItem);
    tcx.infer_ctxt()
       .enter(|infcx| traits::type_known_to_meet_bound(&infcx,
                                                       param_env,
                                                       ty,
                                                       trait_def_id,
                                                       DUMMY_SP))
}

fn is_freeze_raw<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                           query: ty::ParamEnvAnd<'tcx, Ty<'tcx>>)
                           -> bool
{
    let (param_env, ty) = query.into_parts();
    let trait_def_id = tcx.require_lang_item(lang_items::FreezeTraitLangItem);
    tcx.infer_ctxt()
       .enter(|infcx| traits::type_known_to_meet_bound(&infcx,
                                                       param_env,
                                                       ty,
                                                       trait_def_id,
                                                       DUMMY_SP))
}

fn needs_drop_raw<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                            query: ty::ParamEnvAnd<'tcx, Ty<'tcx>>)
                            -> bool
{
    let (param_env, ty) = query.into_parts();

    let needs_drop = |ty: Ty<'tcx>| -> bool {
        match tcx.try_needs_drop_raw(DUMMY_SP, param_env.and(ty)) {
            Ok(v) => v,
            Err(mut bug) => {
                // Cycles should be reported as an error by `check_representable`.
                //
                // Consider the type as not needing drop in the meanwhile to
                // avoid further errors.
                //
                // In case we forgot to emit a bug elsewhere, delay our
                // diagnostic to get emitted as a compiler bug.
                bug.delay_as_bug();
                false
            }
        }
    };

    assert!(!ty.needs_infer());

    match ty.sty {
        // Fast-path for primitive types
        ty::TyInfer(ty::FreshIntTy(_)) | ty::TyInfer(ty::FreshFloatTy(_)) |
        ty::TyBool | ty::TyInt(_) | ty::TyUint(_) | ty::TyFloat(_) | ty::TyNever |
        ty::TyFnDef(..) | ty::TyFnPtr(_) | ty::TyChar | ty::TyGeneratorWitness(..) |
        ty::TyRawPtr(_) | ty::TyRef(..) | ty::TyStr => false,

        // Foreign types can never have destructors
        ty::TyForeign(..) => false,

        // Issue #22536: We first query type_moves_by_default.  It sees a
        // normalized version of the type, and therefore will definitely
        // know whether the type implements Copy (and thus needs no
        // cleanup/drop/zeroing) ...
        _ if !ty.moves_by_default(tcx, param_env, DUMMY_SP) => false,

        // ... (issue #22536 continued) but as an optimization, still use
        // prior logic of asking for the structural "may drop".

        // FIXME(#22815): Note that this is a conservative heuristic;
        // it may report that the type "may drop" when actual type does
        // not actually have a destructor associated with it. But since
        // the type absolutely did not have the `Copy` bound attached
        // (see above), it is sound to treat it as having a destructor.

        // User destructors are the only way to have concrete drop types.
        ty::TyAdt(def, _) if def.has_dtor(tcx) => true,

        // Can refer to a type which may drop.
        // FIXME(eddyb) check this against a ParamEnv.
        ty::TyDynamic(..) | ty::TyProjection(..) | ty::TyParam(_) |
        ty::TyAnon(..) | ty::TyInfer(_) | ty::TyError => true,

        // Structural recursion.
        ty::TyArray(ty, _) | ty::TySlice(ty) => needs_drop(ty),

        ty::TyClosure(def_id, ref substs) => substs.upvar_tys(def_id, tcx).any(needs_drop),

        // Pessimistically assume that all generators will require destructors
        // as we don't know if a destructor is a noop or not until after the MIR
        // state transformation pass
        ty::TyGenerator(..) => true,

        ty::TyTuple(ref tys) => tys.iter().cloned().any(needs_drop),

        // unions don't have destructors regardless of the child types
        ty::TyAdt(def, _) if def.is_union() => false,

        ty::TyAdt(def, substs) =>
            def.variants.iter().any(
                |variant| variant.fields.iter().any(
                    |field| needs_drop(field.ty(tcx, substs)))),
    }
}

pub enum ExplicitSelf<'tcx> {
    ByValue,
    ByReference(ty::Region<'tcx>, hir::Mutability),
    ByRawPointer(hir::Mutability),
    ByBox,
    Other
}

impl<'tcx> ExplicitSelf<'tcx> {
    /// Categorizes an explicit self declaration like `self: SomeType`
    /// into either `self`, `&self`, `&mut self`, `Box<self>`, or
    /// `Other`.
    /// This is mainly used to require the arbitrary_self_types feature
    /// in the case of `Other`, to improve error messages in the common cases,
    /// and to make `Other` non-object-safe.
    ///
    /// Examples:
    ///
    /// ```
    /// impl<'a> Foo for &'a T {
    ///     // Legal declarations:
    ///     fn method1(self: &&'a T); // ExplicitSelf::ByReference
    ///     fn method2(self: &'a T); // ExplicitSelf::ByValue
    ///     fn method3(self: Box<&'a T>); // ExplicitSelf::ByBox
    ///     fn method4(self: Rc<&'a T>); // ExplicitSelf::Other
    ///
    ///     // Invalid cases will be caught by `check_method_receiver`:
    ///     fn method_err1(self: &'a mut T); // ExplicitSelf::Other
    ///     fn method_err2(self: &'static T) // ExplicitSelf::ByValue
    ///     fn method_err3(self: &&T) // ExplicitSelf::ByReference
    /// }
    /// ```
    ///
    pub fn determine<P>(
        self_arg_ty: Ty<'tcx>,
        is_self_ty: P
    ) -> ExplicitSelf<'tcx>
    where
        P: Fn(Ty<'tcx>) -> bool
    {
        use self::ExplicitSelf::*;

        match self_arg_ty.sty {
            _ if is_self_ty(self_arg_ty) => ByValue,
            ty::TyRef(region, ty, mutbl) if is_self_ty(ty) => {
                ByReference(region, mutbl)
            }
            ty::TyRawPtr(ty::TypeAndMut { ty, mutbl }) if is_self_ty(ty) => {
                ByRawPointer(mutbl)
            }
            ty::TyAdt(def, _) if def.is_box() && is_self_ty(self_arg_ty.boxed_ty()) => {
                ByBox
            }
            _ => Other
        }
    }
}

pub fn provide(providers: &mut ty::query::Providers) {
    *providers = ty::query::Providers {
        is_copy_raw,
        is_sized_raw,
        is_freeze_raw,
        needs_drop_raw,
        ..*providers
    };
}
