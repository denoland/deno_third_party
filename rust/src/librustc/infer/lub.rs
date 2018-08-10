// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::combine::CombineFields;
use super::InferCtxt;
use super::lattice::{self, LatticeDir};
use super::Subtype;

use traits::ObligationCause;
use ty::{self, Ty, TyCtxt};
use ty::error::TypeError;
use ty::relate::{Relate, RelateResult, TypeRelation};

/// "Least upper bound" (common supertype)
pub struct Lub<'combine, 'infcx: 'combine, 'gcx: 'infcx+'tcx, 'tcx: 'infcx> {
    fields: &'combine mut CombineFields<'infcx, 'gcx, 'tcx>,
    a_is_expected: bool,
}

impl<'combine, 'infcx, 'gcx, 'tcx> Lub<'combine, 'infcx, 'gcx, 'tcx> {
    pub fn new(fields: &'combine mut CombineFields<'infcx, 'gcx, 'tcx>, a_is_expected: bool)
        -> Lub<'combine, 'infcx, 'gcx, 'tcx>
    {
        Lub { fields: fields, a_is_expected: a_is_expected }
    }
}

impl<'combine, 'infcx, 'gcx, 'tcx> TypeRelation<'infcx, 'gcx, 'tcx>
    for Lub<'combine, 'infcx, 'gcx, 'tcx>
{
    fn tag(&self) -> &'static str { "Lub" }

    fn tcx(&self) -> TyCtxt<'infcx, 'gcx, 'tcx> { self.fields.tcx() }

    fn a_is_expected(&self) -> bool { self.a_is_expected }

    fn relate_with_variance<T: Relate<'tcx>>(&mut self,
                                             variance: ty::Variance,
                                             a: &T,
                                             b: &T)
                                             -> RelateResult<'tcx, T>
    {
        match variance {
            ty::Invariant => self.fields.equate(self.a_is_expected).relate(a, b),
            ty::Covariant => self.relate(a, b),
            // FIXME(#41044) -- not correct, need test
            ty::Bivariant => Ok(a.clone()),
            ty::Contravariant => self.fields.glb(self.a_is_expected).relate(a, b),
        }
    }

    fn tys(&mut self, a: Ty<'tcx>, b: Ty<'tcx>) -> RelateResult<'tcx, Ty<'tcx>> {
        lattice::super_lattice_tys(self, a, b)
    }

    fn regions(&mut self, a: ty::Region<'tcx>, b: ty::Region<'tcx>)
               -> RelateResult<'tcx, ty::Region<'tcx>> {
        debug!("{}.regions({:?}, {:?})",
               self.tag(),
               a,
               b);

        let origin = Subtype(self.fields.trace.clone());
        Ok(self.fields.infcx.borrow_region_constraints().lub_regions(self.tcx(), origin, a, b))
    }

    fn binders<T>(&mut self, a: &ty::Binder<T>, b: &ty::Binder<T>)
                  -> RelateResult<'tcx, ty::Binder<T>>
        where T: Relate<'tcx>
    {
        debug!("binders(a={:?}, b={:?})", a, b);
        let was_error = self.infcx().probe(|_snapshot| {
            // Subtle: use a fresh combine-fields here because we recover
            // from Err. Doing otherwise could propagate obligations out
            // through our `self.obligations` field.
            self.infcx()
                .combine_fields(self.fields.trace.clone(), self.fields.param_env)
                .higher_ranked_lub(a, b, self.a_is_expected)
                .is_err()
        });
        debug!("binders: was_error={:?}", was_error);

        // When higher-ranked types are involved, computing the LUB is
        // very challenging, switch to invariance. This is obviously
        // overly conservative but works ok in practice.
        match self.relate_with_variance(ty::Variance::Invariant, a, b) {
            Ok(_) => Ok(a.clone()),
            Err(err) => {
                debug!("binders: error occurred, was_error={:?}", was_error);
                if !was_error {
                    Err(TypeError::OldStyleLUB(Box::new(err)))
                } else {
                    Err(err)
                }
            }
        }
    }
}

impl<'combine, 'infcx, 'gcx, 'tcx> LatticeDir<'infcx, 'gcx, 'tcx>
    for Lub<'combine, 'infcx, 'gcx, 'tcx>
{
    fn infcx(&self) -> &'infcx InferCtxt<'infcx, 'gcx, 'tcx> {
        self.fields.infcx
    }

    fn cause(&self) -> &ObligationCause<'tcx> {
        &self.fields.trace.cause
    }

    fn relate_bound(&mut self, v: Ty<'tcx>, a: Ty<'tcx>, b: Ty<'tcx>) -> RelateResult<'tcx, ()> {
        let mut sub = self.fields.sub(self.a_is_expected);
        sub.relate(&a, &v)?;
        sub.relate(&b, &v)?;
        Ok(())
    }
}
