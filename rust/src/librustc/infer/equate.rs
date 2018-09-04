// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::combine::{CombineFields, RelationDir};
use super::{Subtype};

use hir::def_id::DefId;

use ty::{self, Ty, TyCtxt};
use ty::TyVar;
use ty::subst::Substs;
use ty::relate::{self, Relate, RelateResult, TypeRelation};

/// Ensures `a` is made equal to `b`. Returns `a` on success.
pub struct Equate<'combine, 'infcx: 'combine, 'gcx: 'infcx+'tcx, 'tcx: 'infcx> {
    fields: &'combine mut CombineFields<'infcx, 'gcx, 'tcx>,
    a_is_expected: bool,
}

impl<'combine, 'infcx, 'gcx, 'tcx> Equate<'combine, 'infcx, 'gcx, 'tcx> {
    pub fn new(fields: &'combine mut CombineFields<'infcx, 'gcx, 'tcx>, a_is_expected: bool)
        -> Equate<'combine, 'infcx, 'gcx, 'tcx>
    {
        Equate { fields: fields, a_is_expected: a_is_expected }
    }
}

impl<'combine, 'infcx, 'gcx, 'tcx> TypeRelation<'infcx, 'gcx, 'tcx>
    for Equate<'combine, 'infcx, 'gcx, 'tcx>
{
    fn tag(&self) -> &'static str { "Equate" }

    fn tcx(&self) -> TyCtxt<'infcx, 'gcx, 'tcx> { self.fields.tcx() }

    fn a_is_expected(&self) -> bool { self.a_is_expected }

    fn relate_item_substs(&mut self,
                          _item_def_id: DefId,
                          a_subst: &'tcx Substs<'tcx>,
                          b_subst: &'tcx Substs<'tcx>)
                          -> RelateResult<'tcx, &'tcx Substs<'tcx>>
    {
        // NB: Once we are equating types, we don't care about
        // variance, so don't try to lookup the variance here. This
        // also avoids some cycles (e.g. #41849) since looking up
        // variance requires computing types which can require
        // performing trait matching (which then performs equality
        // unification).

        relate::relate_substs(self, None, a_subst, b_subst)
    }

    fn relate_with_variance<T: Relate<'tcx>>(&mut self,
                                             _: ty::Variance,
                                             a: &T,
                                             b: &T)
                                             -> RelateResult<'tcx, T>
    {
        self.relate(a, b)
    }

    fn tys(&mut self, a: Ty<'tcx>, b: Ty<'tcx>) -> RelateResult<'tcx, Ty<'tcx>> {
        debug!("{}.tys({:?}, {:?})", self.tag(),
               a, b);
        if a == b { return Ok(a); }

        let infcx = self.fields.infcx;
        let a = infcx.type_variables.borrow_mut().replace_if_possible(a);
        let b = infcx.type_variables.borrow_mut().replace_if_possible(b);
        match (&a.sty, &b.sty) {
            (&ty::TyInfer(TyVar(a_id)), &ty::TyInfer(TyVar(b_id))) => {
                infcx.type_variables.borrow_mut().equate(a_id, b_id);
                Ok(a)
            }

            (&ty::TyInfer(TyVar(a_id)), _) => {
                self.fields.instantiate(b, RelationDir::EqTo, a_id, self.a_is_expected)?;
                Ok(a)
            }

            (_, &ty::TyInfer(TyVar(b_id))) => {
                self.fields.instantiate(a, RelationDir::EqTo, b_id, self.a_is_expected)?;
                Ok(a)
            }

            _ => {
                self.fields.infcx.super_combine_tys(self, a, b)?;
                Ok(a)
            }
        }
    }

    fn regions(&mut self, a: ty::Region<'tcx>, b: ty::Region<'tcx>)
               -> RelateResult<'tcx, ty::Region<'tcx>> {
        debug!("{}.regions({:?}, {:?})",
               self.tag(),
               a,
               b);
        let origin = Subtype(self.fields.trace.clone());
        self.fields.infcx.borrow_region_constraints()
                         .make_eqregion(origin, a, b);
        Ok(a)
    }

    fn binders<T>(&mut self, a: &ty::Binder<T>, b: &ty::Binder<T>)
                  -> RelateResult<'tcx, ty::Binder<T>>
        where T: Relate<'tcx>
    {
        self.fields.higher_ranked_sub(a, b, self.a_is_expected)?;
        self.fields.higher_ranked_sub(b, a, self.a_is_expected)
    }
}
