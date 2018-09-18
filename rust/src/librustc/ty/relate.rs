// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Generalized type relating mechanism. A type relation R relates a
//! pair of values (A, B). A and B are usually types or regions but
//! can be other things. Examples of type relations are subtyping,
//! type equality, etc.

use hir::def_id::DefId;
use middle::const_val::ConstVal;
use ty::subst::{Kind, UnpackedKind, Substs};
use ty::{self, Ty, TyCtxt, TypeFoldable};
use ty::error::{ExpectedFound, TypeError};
use mir::interpret::GlobalId;
use util::common::ErrorReported;
use std::rc::Rc;
use std::iter;
use rustc_target::spec::abi;
use hir as ast;

pub type RelateResult<'tcx, T> = Result<T, TypeError<'tcx>>;

#[derive(Clone, Debug)]
pub enum Cause {
    ExistentialRegionBound, // relating an existential region bound
}

pub trait TypeRelation<'a, 'gcx: 'a+'tcx, 'tcx: 'a> : Sized {
    fn tcx(&self) -> TyCtxt<'a, 'gcx, 'tcx>;

    /// Returns a static string we can use for printouts.
    fn tag(&self) -> &'static str;

    /// Returns true if the value `a` is the "expected" type in the
    /// relation. Just affects error messages.
    fn a_is_expected(&self) -> bool;

    fn with_cause<F,R>(&mut self, _cause: Cause, f: F) -> R
        where F: FnOnce(&mut Self) -> R
    {
        f(self)
    }

    /// Generic relation routine suitable for most anything.
    fn relate<T: Relate<'tcx>>(&mut self, a: &T, b: &T) -> RelateResult<'tcx, T> {
        Relate::relate(self, a, b)
    }

    /// Relate the two substitutions for the given item. The default
    /// is to look up the variance for the item and proceed
    /// accordingly.
    fn relate_item_substs(&mut self,
                          item_def_id: DefId,
                          a_subst: &'tcx Substs<'tcx>,
                          b_subst: &'tcx Substs<'tcx>)
                          -> RelateResult<'tcx, &'tcx Substs<'tcx>>
    {
        debug!("relate_item_substs(item_def_id={:?}, a_subst={:?}, b_subst={:?})",
               item_def_id,
               a_subst,
               b_subst);

        let opt_variances = self.tcx().variances_of(item_def_id);
        relate_substs(self, Some(&opt_variances), a_subst, b_subst)
    }

    /// Switch variance for the purpose of relating `a` and `b`.
    fn relate_with_variance<T: Relate<'tcx>>(&mut self,
                                             variance: ty::Variance,
                                             a: &T,
                                             b: &T)
                                             -> RelateResult<'tcx, T>;

    // Overrideable relations. You shouldn't typically call these
    // directly, instead call `relate()`, which in turn calls
    // these. This is both more uniform but also allows us to add
    // additional hooks for other types in the future if needed
    // without making older code, which called `relate`, obsolete.

    fn tys(&mut self, a: Ty<'tcx>, b: Ty<'tcx>)
           -> RelateResult<'tcx, Ty<'tcx>>;

    fn regions(&mut self, a: ty::Region<'tcx>, b: ty::Region<'tcx>)
               -> RelateResult<'tcx, ty::Region<'tcx>>;

    fn binders<T>(&mut self, a: &ty::Binder<T>, b: &ty::Binder<T>)
                  -> RelateResult<'tcx, ty::Binder<T>>
        where T: Relate<'tcx>;
}

pub trait Relate<'tcx>: TypeFoldable<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R, a: &Self, b: &Self)
                           -> RelateResult<'tcx, Self>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a;
}

///////////////////////////////////////////////////////////////////////////
// Relate impls

impl<'tcx> Relate<'tcx> for ty::TypeAndMut<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::TypeAndMut<'tcx>,
                           b: &ty::TypeAndMut<'tcx>)
                           -> RelateResult<'tcx, ty::TypeAndMut<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        debug!("{}.mts({:?}, {:?})",
               relation.tag(),
               a,
               b);
        if a.mutbl != b.mutbl {
            Err(TypeError::Mutability)
        } else {
            let mutbl = a.mutbl;
            let variance = match mutbl {
                ast::Mutability::MutImmutable => ty::Covariant,
                ast::Mutability::MutMutable => ty::Invariant,
            };
            let ty = relation.relate_with_variance(variance, &a.ty, &b.ty)?;
            Ok(ty::TypeAndMut {ty: ty, mutbl: mutbl})
        }
    }
}

pub fn relate_substs<'a, 'gcx, 'tcx, R>(relation: &mut R,
                                        variances: Option<&Vec<ty::Variance>>,
                                        a_subst: &'tcx Substs<'tcx>,
                                        b_subst: &'tcx Substs<'tcx>)
                                        -> RelateResult<'tcx, &'tcx Substs<'tcx>>
    where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
{
    let tcx = relation.tcx();

    let params = a_subst.iter().zip(b_subst).enumerate().map(|(i, (a, b))| {
        let variance = variances.map_or(ty::Invariant, |v| v[i]);
        relation.relate_with_variance(variance, a, b)
    });

    Ok(tcx.mk_substs(params)?)
}

impl<'tcx> Relate<'tcx> for ty::FnSig<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::FnSig<'tcx>,
                           b: &ty::FnSig<'tcx>)
                           -> RelateResult<'tcx, ty::FnSig<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        let tcx = relation.tcx();

        if a.variadic != b.variadic {
            return Err(TypeError::VariadicMismatch(
                expected_found(relation, &a.variadic, &b.variadic)));
        }
        let unsafety = relation.relate(&a.unsafety, &b.unsafety)?;
        let abi = relation.relate(&a.abi, &b.abi)?;

        if a.inputs().len() != b.inputs().len() {
            return Err(TypeError::ArgCount);
        }

        let inputs_and_output = a.inputs().iter().cloned()
            .zip(b.inputs().iter().cloned())
            .map(|x| (x, false))
            .chain(iter::once(((a.output(), b.output()), true)))
            .map(|((a, b), is_output)| {
                if is_output {
                    relation.relate(&a, &b)
                } else {
                    relation.relate_with_variance(ty::Contravariant, &a, &b)
                }
            });
        Ok(ty::FnSig {
            inputs_and_output: tcx.mk_type_list(inputs_and_output)?,
            variadic: a.variadic,
            unsafety,
            abi,
        })
    }
}

impl<'tcx> Relate<'tcx> for ast::Unsafety {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ast::Unsafety,
                           b: &ast::Unsafety)
                           -> RelateResult<'tcx, ast::Unsafety>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        if a != b {
            Err(TypeError::UnsafetyMismatch(expected_found(relation, a, b)))
        } else {
            Ok(*a)
        }
    }
}

impl<'tcx> Relate<'tcx> for abi::Abi {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &abi::Abi,
                           b: &abi::Abi)
                           -> RelateResult<'tcx, abi::Abi>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        if a == b {
            Ok(*a)
        } else {
            Err(TypeError::AbiMismatch(expected_found(relation, a, b)))
        }
    }
}

impl<'tcx> Relate<'tcx> for ty::ProjectionTy<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::ProjectionTy<'tcx>,
                           b: &ty::ProjectionTy<'tcx>)
                           -> RelateResult<'tcx, ty::ProjectionTy<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        if a.item_def_id != b.item_def_id {
            Err(TypeError::ProjectionMismatched(
                expected_found(relation, &a.item_def_id, &b.item_def_id)))
        } else {
            let substs = relation.relate(&a.substs, &b.substs)?;
            Ok(ty::ProjectionTy {
                item_def_id: a.item_def_id,
                substs: &substs,
            })
        }
    }
}

impl<'tcx> Relate<'tcx> for ty::ExistentialProjection<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::ExistentialProjection<'tcx>,
                           b: &ty::ExistentialProjection<'tcx>)
                           -> RelateResult<'tcx, ty::ExistentialProjection<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        if a.item_def_id != b.item_def_id {
            Err(TypeError::ProjectionMismatched(
                expected_found(relation, &a.item_def_id, &b.item_def_id)))
        } else {
            let ty = relation.relate(&a.ty, &b.ty)?;
            let substs = relation.relate(&a.substs, &b.substs)?;
            Ok(ty::ExistentialProjection {
                item_def_id: a.item_def_id,
                substs,
                ty,
            })
        }
    }
}

impl<'tcx> Relate<'tcx> for Vec<ty::PolyExistentialProjection<'tcx>> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &Vec<ty::PolyExistentialProjection<'tcx>>,
                           b: &Vec<ty::PolyExistentialProjection<'tcx>>)
                           -> RelateResult<'tcx, Vec<ty::PolyExistentialProjection<'tcx>>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        // To be compatible, `a` and `b` must be for precisely the
        // same set of traits and item names. We always require that
        // projection bounds lists are sorted by trait-def-id and item-name,
        // so we can just iterate through the lists pairwise, so long as they are the
        // same length.
        if a.len() != b.len() {
            Err(TypeError::ProjectionBoundsLength(expected_found(relation, &a.len(), &b.len())))
        } else {
            a.iter().zip(b)
                .map(|(a, b)| relation.relate(a, b))
                .collect()
        }
    }
}

impl<'tcx> Relate<'tcx> for ty::TraitRef<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::TraitRef<'tcx>,
                           b: &ty::TraitRef<'tcx>)
                           -> RelateResult<'tcx, ty::TraitRef<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        // Different traits cannot be related
        if a.def_id != b.def_id {
            Err(TypeError::Traits(expected_found(relation, &a.def_id, &b.def_id)))
        } else {
            let substs = relate_substs(relation, None, a.substs, b.substs)?;
            Ok(ty::TraitRef { def_id: a.def_id, substs: substs })
        }
    }
}

impl<'tcx> Relate<'tcx> for ty::ExistentialTraitRef<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::ExistentialTraitRef<'tcx>,
                           b: &ty::ExistentialTraitRef<'tcx>)
                           -> RelateResult<'tcx, ty::ExistentialTraitRef<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        // Different traits cannot be related
        if a.def_id != b.def_id {
            Err(TypeError::Traits(expected_found(relation, &a.def_id, &b.def_id)))
        } else {
            let substs = relate_substs(relation, None, a.substs, b.substs)?;
            Ok(ty::ExistentialTraitRef { def_id: a.def_id, substs: substs })
        }
    }
}

#[derive(Debug, Clone)]
struct GeneratorWitness<'tcx>(&'tcx ty::Slice<Ty<'tcx>>);

TupleStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for GeneratorWitness<'tcx> {
        a
    }
}

impl<'tcx> Relate<'tcx> for GeneratorWitness<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &GeneratorWitness<'tcx>,
                           b: &GeneratorWitness<'tcx>)
                           -> RelateResult<'tcx, GeneratorWitness<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        assert!(a.0.len() == b.0.len());
        let tcx = relation.tcx();
        let types = tcx.mk_type_list(a.0.iter().zip(b.0).map(|(a, b)| relation.relate(a, b)))?;
        Ok(GeneratorWitness(types))
    }
}

impl<'tcx> Relate<'tcx> for Ty<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &Ty<'tcx>,
                           b: &Ty<'tcx>)
                           -> RelateResult<'tcx, Ty<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        relation.tys(a, b)
    }
}

/// The main "type relation" routine. Note that this does not handle
/// inference artifacts, so you should filter those out before calling
/// it.
pub fn super_relate_tys<'a, 'gcx, 'tcx, R>(relation: &mut R,
                                           a: Ty<'tcx>,
                                           b: Ty<'tcx>)
                                           -> RelateResult<'tcx, Ty<'tcx>>
    where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
{
    let tcx = relation.tcx();
    let a_sty = &a.sty;
    let b_sty = &b.sty;
    debug!("super_tys: a_sty={:?} b_sty={:?}", a_sty, b_sty);
    match (a_sty, b_sty) {
        (&ty::TyInfer(_), _) |
        (_, &ty::TyInfer(_)) =>
        {
            // The caller should handle these cases!
            bug!("var types encountered in super_relate_tys")
        }

        (&ty::TyError, _) | (_, &ty::TyError) =>
        {
            Ok(tcx.types.err)
        }

        (&ty::TyNever, _) |
        (&ty::TyChar, _) |
        (&ty::TyBool, _) |
        (&ty::TyInt(_), _) |
        (&ty::TyUint(_), _) |
        (&ty::TyFloat(_), _) |
        (&ty::TyStr, _)
            if a == b =>
        {
            Ok(a)
        }

        (&ty::TyParam(ref a_p), &ty::TyParam(ref b_p))
            if a_p.idx == b_p.idx =>
        {
            Ok(a)
        }

        (&ty::TyAdt(a_def, a_substs), &ty::TyAdt(b_def, b_substs))
            if a_def == b_def =>
        {
            let substs = relation.relate_item_substs(a_def.did, a_substs, b_substs)?;
            Ok(tcx.mk_adt(a_def, substs))
        }

        (&ty::TyForeign(a_id), &ty::TyForeign(b_id))
            if a_id == b_id =>
        {
            Ok(tcx.mk_foreign(a_id))
        }

        (&ty::TyDynamic(ref a_obj, ref a_region), &ty::TyDynamic(ref b_obj, ref b_region)) => {
            let region_bound = relation.with_cause(Cause::ExistentialRegionBound,
                                                       |relation| {
                                                           relation.relate_with_variance(
                                                               ty::Contravariant,
                                                               a_region,
                                                               b_region)
                                                       })?;
            Ok(tcx.mk_dynamic(relation.relate(a_obj, b_obj)?, region_bound))
        }

        (&ty::TyGenerator(a_id, a_substs, movability),
         &ty::TyGenerator(b_id, b_substs, _))
            if a_id == b_id =>
        {
            // All TyGenerator types with the same id represent
            // the (anonymous) type of the same generator expression. So
            // all of their regions should be equated.
            let substs = relation.relate(&a_substs, &b_substs)?;
            Ok(tcx.mk_generator(a_id, substs, movability))
        }

        (&ty::TyGeneratorWitness(a_types), &ty::TyGeneratorWitness(b_types)) =>
        {
            // Wrap our types with a temporary GeneratorWitness struct
            // inside the binder so we can related them
            let a_types = a_types.map_bound(GeneratorWitness);
            let b_types = b_types.map_bound(GeneratorWitness);
            // Then remove the GeneratorWitness for the result
            let types = relation.relate(&a_types, &b_types)?.map_bound(|witness| witness.0);
            Ok(tcx.mk_generator_witness(types))
        }

        (&ty::TyClosure(a_id, a_substs),
         &ty::TyClosure(b_id, b_substs))
            if a_id == b_id =>
        {
            // All TyClosure types with the same id represent
            // the (anonymous) type of the same closure expression. So
            // all of their regions should be equated.
            let substs = relation.relate(&a_substs, &b_substs)?;
            Ok(tcx.mk_closure(a_id, substs))
        }

        (&ty::TyRawPtr(ref a_mt), &ty::TyRawPtr(ref b_mt)) =>
        {
            let mt = relation.relate(a_mt, b_mt)?;
            Ok(tcx.mk_ptr(mt))
        }

        (&ty::TyRef(a_r, a_ty, a_mutbl), &ty::TyRef(b_r, b_ty, b_mutbl)) =>
        {
            let r = relation.relate_with_variance(ty::Contravariant, &a_r, &b_r)?;
            let a_mt = ty::TypeAndMut { ty: a_ty, mutbl: a_mutbl };
            let b_mt = ty::TypeAndMut { ty: b_ty, mutbl: b_mutbl };
            let mt = relation.relate(&a_mt, &b_mt)?;
            Ok(tcx.mk_ref(r, mt))
        }

        (&ty::TyArray(a_t, sz_a), &ty::TyArray(b_t, sz_b)) =>
        {
            let t = relation.relate(&a_t, &b_t)?;
            assert_eq!(sz_a.ty, tcx.types.usize);
            assert_eq!(sz_b.ty, tcx.types.usize);
            let to_u64 = |x: &'tcx ty::Const<'tcx>| -> Result<u64, ErrorReported> {
                if let Some(s) = x.assert_usize(tcx) {
                    return Ok(s);
                }
                match x.val {
                    ConstVal::Unevaluated(def_id, substs) => {
                        // FIXME(eddyb) get the right param_env.
                        let param_env = ty::ParamEnv::empty();
                        match tcx.lift_to_global(&substs) {
                            Some(substs) => {
                                let instance = ty::Instance::resolve(
                                    tcx.global_tcx(),
                                    param_env,
                                    def_id,
                                    substs,
                                );
                                if let Some(instance) = instance {
                                    let cid = GlobalId {
                                        instance,
                                        promoted: None
                                    };
                                    if let Some(s) = tcx.const_eval(param_env.and(cid))
                                                        .ok()
                                                        .map(|c| c.unwrap_usize(tcx)) {
                                        return Ok(s)
                                    }
                                }
                            },
                            None => {}
                        }
                        tcx.sess.delay_span_bug(tcx.def_span(def_id),
                            "array length could not be evaluated");
                        Err(ErrorReported)
                    }
                    _ => bug!("arrays should not have {:?} as length", x)
                }
            };
            match (to_u64(sz_a), to_u64(sz_b)) {
                (Ok(sz_a_u64), Ok(sz_b_u64)) => {
                    if sz_a_u64 == sz_b_u64 {
                        Ok(tcx.mk_ty(ty::TyArray(t, sz_a)))
                    } else {
                        Err(TypeError::FixedArraySize(
                            expected_found(relation, &sz_a_u64, &sz_b_u64)))
                    }
                }
                // We reported an error or will ICE, so we can return TyError.
                (Err(ErrorReported), _) | (_, Err(ErrorReported)) => {
                    Ok(tcx.types.err)
                }
            }
        }

        (&ty::TySlice(a_t), &ty::TySlice(b_t)) =>
        {
            let t = relation.relate(&a_t, &b_t)?;
            Ok(tcx.mk_slice(t))
        }

        (&ty::TyTuple(as_), &ty::TyTuple(bs)) =>
        {
            if as_.len() == bs.len() {
                Ok(tcx.mk_tup(as_.iter().zip(bs).map(|(a, b)| relation.relate(a, b)))?)
            } else if !(as_.is_empty() || bs.is_empty()) {
                Err(TypeError::TupleSize(
                    expected_found(relation, &as_.len(), &bs.len())))
            } else {
                Err(TypeError::Sorts(expected_found(relation, &a, &b)))
            }
        }

        (&ty::TyFnDef(a_def_id, a_substs), &ty::TyFnDef(b_def_id, b_substs))
            if a_def_id == b_def_id =>
        {
            let substs = relation.relate_item_substs(a_def_id, a_substs, b_substs)?;
            Ok(tcx.mk_fn_def(a_def_id, substs))
        }

        (&ty::TyFnPtr(a_fty), &ty::TyFnPtr(b_fty)) =>
        {
            let fty = relation.relate(&a_fty, &b_fty)?;
            Ok(tcx.mk_fn_ptr(fty))
        }

        (&ty::TyProjection(ref a_data), &ty::TyProjection(ref b_data)) =>
        {
            let projection_ty = relation.relate(a_data, b_data)?;
            Ok(tcx.mk_projection(projection_ty.item_def_id, projection_ty.substs))
        }

        (&ty::TyAnon(a_def_id, a_substs), &ty::TyAnon(b_def_id, b_substs))
            if a_def_id == b_def_id =>
        {
            let substs = relate_substs(relation, None, a_substs, b_substs)?;
            Ok(tcx.mk_anon(a_def_id, substs))
        }

        _ =>
        {
            Err(TypeError::Sorts(expected_found(relation, &a, &b)))
        }
    }
}

impl<'tcx> Relate<'tcx> for &'tcx ty::Slice<ty::ExistentialPredicate<'tcx>> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &Self,
                           b: &Self)
        -> RelateResult<'tcx, Self>
            where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a {

        if a.len() != b.len() {
            return Err(TypeError::ExistentialMismatch(expected_found(relation, a, b)));
        }

        let tcx = relation.tcx();
        let v = a.iter().zip(b.iter()).map(|(ep_a, ep_b)| {
            use ty::ExistentialPredicate::*;
            match (*ep_a, *ep_b) {
                (Trait(ref a), Trait(ref b)) => Ok(Trait(relation.relate(a, b)?)),
                (Projection(ref a), Projection(ref b)) => Ok(Projection(relation.relate(a, b)?)),
                (AutoTrait(ref a), AutoTrait(ref b)) if a == b => Ok(AutoTrait(*a)),
                _ => Err(TypeError::ExistentialMismatch(expected_found(relation, a, b)))
            }
        });
        Ok(tcx.mk_existential_predicates(v)?)
    }
}

impl<'tcx> Relate<'tcx> for ty::ClosureSubsts<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::ClosureSubsts<'tcx>,
                           b: &ty::ClosureSubsts<'tcx>)
                           -> RelateResult<'tcx, ty::ClosureSubsts<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        let substs = relate_substs(relation, None, a.substs, b.substs)?;
        Ok(ty::ClosureSubsts { substs })
    }
}

impl<'tcx> Relate<'tcx> for ty::GeneratorSubsts<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::GeneratorSubsts<'tcx>,
                           b: &ty::GeneratorSubsts<'tcx>)
                           -> RelateResult<'tcx, ty::GeneratorSubsts<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        let substs = relate_substs(relation, None, a.substs, b.substs)?;
        Ok(ty::GeneratorSubsts { substs })
    }
}

impl<'tcx> Relate<'tcx> for &'tcx Substs<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &&'tcx Substs<'tcx>,
                           b: &&'tcx Substs<'tcx>)
                           -> RelateResult<'tcx, &'tcx Substs<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        relate_substs(relation, None, a, b)
    }
}

impl<'tcx> Relate<'tcx> for ty::Region<'tcx> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::Region<'tcx>,
                           b: &ty::Region<'tcx>)
                           -> RelateResult<'tcx, ty::Region<'tcx>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        relation.regions(*a, *b)
    }
}

impl<'tcx, T: Relate<'tcx>> Relate<'tcx> for ty::Binder<T> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &ty::Binder<T>,
                           b: &ty::Binder<T>)
                           -> RelateResult<'tcx, ty::Binder<T>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        relation.binders(a, b)
    }
}

impl<'tcx, T: Relate<'tcx>> Relate<'tcx> for Rc<T> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &Rc<T>,
                           b: &Rc<T>)
                           -> RelateResult<'tcx, Rc<T>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        let a: &T = a;
        let b: &T = b;
        Ok(Rc::new(relation.relate(a, b)?))
    }
}

impl<'tcx, T: Relate<'tcx>> Relate<'tcx> for Box<T> {
    fn relate<'a, 'gcx, R>(relation: &mut R,
                           a: &Box<T>,
                           b: &Box<T>)
                           -> RelateResult<'tcx, Box<T>>
        where R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a
    {
        let a: &T = a;
        let b: &T = b;
        Ok(Box::new(relation.relate(a, b)?))
    }
}

impl<'tcx> Relate<'tcx> for Kind<'tcx> {
    fn relate<'a, 'gcx, R>(
        relation: &mut R,
        a: &Kind<'tcx>,
        b: &Kind<'tcx>
    ) -> RelateResult<'tcx, Kind<'tcx>>
    where
        R: TypeRelation<'a, 'gcx, 'tcx>, 'gcx: 'a+'tcx, 'tcx: 'a,
    {
        match (a.unpack(), b.unpack()) {
            (UnpackedKind::Lifetime(a_lt), UnpackedKind::Lifetime(b_lt)) => {
                Ok(relation.relate(&a_lt, &b_lt)?.into())
            }
            (UnpackedKind::Type(a_ty), UnpackedKind::Type(b_ty)) => {
                Ok(relation.relate(&a_ty, &b_ty)?.into())
            }
            (UnpackedKind::Lifetime(_), _) | (UnpackedKind::Type(_), _) => bug!()
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Error handling

pub fn expected_found<'a, 'gcx, 'tcx, R, T>(relation: &mut R,
                                            a: &T,
                                            b: &T)
                                            -> ExpectedFound<T>
    where R: TypeRelation<'a, 'gcx, 'tcx>, T: Clone, 'gcx: 'a+'tcx, 'tcx: 'a
{
    expected_found_bool(relation.a_is_expected(), a, b)
}

pub fn expected_found_bool<T>(a_is_expected: bool,
                              a: &T,
                              b: &T)
                              -> ExpectedFound<T>
    where T: Clone
{
    let a = a.clone();
    let b = b.clone();
    if a_is_expected {
        ExpectedFound {expected: a, found: b}
    } else {
        ExpectedFound {expected: b, found: a}
    }
}
