// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Freshening is the process of replacing unknown variables with fresh types. The idea is that
//! the type, after freshening, contains no inference variables but instead contains either a
//! value for each variable or fresh "arbitrary" types wherever a variable would have been.
//!
//! Freshening is used primarily to get a good type for inserting into a cache. The result
//! summarizes what the type inferencer knows "so far". The primary place it is used right now is
//! in the trait matching algorithm, which needs to be able to cache whether an `impl` self type
//! matches some other type X -- *without* affecting `X`. That means if that if the type `X` is in
//! fact an unbound type variable, we want the match to be regarded as ambiguous, because depending
//! on what type that type variable is ultimately assigned, the match may or may not succeed.
//!
//! To handle closures, freshened types also have to contain the signature and kind of any
//! closure in the local inference context, as otherwise the cache key might be invalidated.
//! The way this is done is somewhat hacky - the closure signature is appended to the substs,
//! as well as the closure kind "encoded" as a type. Also, special handling is needed when
//! the closure signature contains a reference to the original closure.
//!
//! Note that you should be careful not to allow the output of freshening to leak to the user in
//! error messages or in any other form. Freshening is only really useful as an internal detail.
//!
//! Because of the manipulation required to handle closures, doing arbitrary operations on
//! freshened types is not recommended. However, in addition to doing equality/hash
//! comparisons (for caching), it is possible to do a `ty::_match` operation between
//! 2 freshened types - this works even with the closure encoding.
//!
//! __An important detail concerning regions.__ The freshener also replaces *all* free regions with
//! 'erased. The reason behind this is that, in general, we do not take region relationships into
//! account when making type-overloaded decisions. This is important because of the design of the
//! region inferencer, which is not based on unification but rather on accumulating and then
//! solving a set of constraints. In contrast, the type inferencer assigns a value to each type
//! variable only once, and it does so as soon as it can, so it is reasonable to ask what the type
//! inferencer knows "so far".

use ty::{self, Ty, TyCtxt, TypeFoldable};
use ty::fold::TypeFolder;
use util::nodemap::FxHashMap;

use std::collections::hash_map::Entry;

use super::InferCtxt;
use super::unify_key::ToType;

pub struct TypeFreshener<'a, 'gcx: 'a+'tcx, 'tcx: 'a> {
    infcx: &'a InferCtxt<'a, 'gcx, 'tcx>,
    freshen_count: u32,
    freshen_map: FxHashMap<ty::InferTy, Ty<'tcx>>,
}

impl<'a, 'gcx, 'tcx> TypeFreshener<'a, 'gcx, 'tcx> {
    pub fn new(infcx: &'a InferCtxt<'a, 'gcx, 'tcx>)
               -> TypeFreshener<'a, 'gcx, 'tcx> {
        TypeFreshener {
            infcx,
            freshen_count: 0,
            freshen_map: FxHashMap(),
        }
    }

    fn freshen<F>(&mut self,
                  opt_ty: Option<Ty<'tcx>>,
                  key: ty::InferTy,
                  freshener: F)
                  -> Ty<'tcx> where
        F: FnOnce(u32) -> ty::InferTy,
    {
        if let Some(ty) = opt_ty {
            return ty.fold_with(self);
        }

        match self.freshen_map.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let index = self.freshen_count;
                self.freshen_count += 1;
                let t = self.infcx.tcx.mk_infer(freshener(index));
                entry.insert(t);
                t
            }
        }
    }
}

impl<'a, 'gcx, 'tcx> TypeFolder<'gcx, 'tcx> for TypeFreshener<'a, 'gcx, 'tcx> {
    fn tcx<'b>(&'b self) -> TyCtxt<'b, 'gcx, 'tcx> {
        self.infcx.tcx
    }

    fn fold_region(&mut self, r: ty::Region<'tcx>) -> ty::Region<'tcx> {
        match *r {
            ty::ReLateBound(..) => {
                // leave bound regions alone
                r
            }

            ty::ReStatic |
            ty::ReEarlyBound(..) |
            ty::ReFree(_) |
            ty::ReScope(_) |
            ty::ReVar(_) |
            ty::ReSkolemized(..) |
            ty::ReEmpty |
            ty::ReErased => {
                // replace all free regions with 'erased
                self.tcx().types.re_erased
            }

            ty::ReCanonical(..) |
            ty::ReClosureBound(..) => {
                bug!(
                    "encountered unexpected region: {:?}",
                    r,
                );
            }
        }
    }

    fn fold_ty(&mut self, t: Ty<'tcx>) -> Ty<'tcx> {
        if !t.needs_infer() && !t.has_erasable_regions() &&
            !(t.has_closure_types() && self.infcx.in_progress_tables.is_some()) {
            return t;
        }

        let tcx = self.infcx.tcx;

        match t.sty {
            ty::TyInfer(ty::TyVar(v)) => {
                let opt_ty = self.infcx.type_variables.borrow_mut().probe(v).known();
                self.freshen(
                    opt_ty,
                    ty::TyVar(v),
                    ty::FreshTy)
            }

            ty::TyInfer(ty::IntVar(v)) => {
                self.freshen(
                    self.infcx.int_unification_table.borrow_mut()
                                                    .probe_value(v)
                                                    .map(|v| v.to_type(tcx)),
                    ty::IntVar(v),
                    ty::FreshIntTy)
            }

            ty::TyInfer(ty::FloatVar(v)) => {
                self.freshen(
                    self.infcx.float_unification_table.borrow_mut()
                                                      .probe_value(v)
                                                      .map(|v| v.to_type(tcx)),
                    ty::FloatVar(v),
                    ty::FreshFloatTy)
            }

            ty::TyInfer(ty::FreshTy(c)) |
            ty::TyInfer(ty::FreshIntTy(c)) |
            ty::TyInfer(ty::FreshFloatTy(c)) => {
                if c >= self.freshen_count {
                    bug!("Encountered a freshend type with id {} \
                          but our counter is only at {}",
                         c,
                         self.freshen_count);
                }
                t
            }

            ty::TyInfer(ty::CanonicalTy(..)) =>
                bug!("encountered canonical ty during freshening"),

            ty::TyGenerator(..) |
            ty::TyBool |
            ty::TyChar |
            ty::TyInt(..) |
            ty::TyUint(..) |
            ty::TyFloat(..) |
            ty::TyAdt(..) |
            ty::TyStr |
            ty::TyError |
            ty::TyArray(..) |
            ty::TySlice(..) |
            ty::TyRawPtr(..) |
            ty::TyRef(..) |
            ty::TyFnDef(..) |
            ty::TyFnPtr(_) |
            ty::TyDynamic(..) |
            ty::TyNever |
            ty::TyTuple(..) |
            ty::TyProjection(..) |
            ty::TyForeign(..) |
            ty::TyParam(..) |
            ty::TyClosure(..) |
            ty::TyGeneratorWitness(..) |
            ty::TyAnon(..) => {
                t.super_fold_with(self)
            }
        }
    }
}
