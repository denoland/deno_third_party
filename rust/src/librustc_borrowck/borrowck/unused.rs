// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rustc::hir::intravisit::{Visitor, NestedVisitorMap};
use rustc::hir::{self, HirId};
use rustc::lint::builtin::UNUSED_MUT;
use rustc::ty;
use rustc::util::nodemap::{FxHashMap, FxHashSet};
use std::slice;
use syntax::ptr::P;

use borrowck::BorrowckCtxt;

pub fn check<'a, 'tcx>(bccx: &BorrowckCtxt<'a, 'tcx>, body: &'tcx hir::Body) {
    let mut used_mut = bccx.used_mut_nodes.borrow().clone();
    UsedMutFinder {
        bccx,
        set: &mut used_mut,
    }.visit_expr(&body.value);
    let mut cx = UnusedMutCx { bccx, used_mut };
    for arg in body.arguments.iter() {
        cx.check_unused_mut_pat(slice::from_ref(&arg.pat));
    }
    cx.visit_expr(&body.value);
}

struct UsedMutFinder<'a, 'tcx: 'a> {
    bccx: &'a BorrowckCtxt<'a, 'tcx>,
    set: &'a mut FxHashSet<HirId>,
}

struct UnusedMutCx<'a, 'tcx: 'a> {
    bccx: &'a BorrowckCtxt<'a, 'tcx>,
    used_mut: FxHashSet<HirId>,
}

impl<'a, 'tcx> UnusedMutCx<'a, 'tcx> {
    fn check_unused_mut_pat(&self, pats: &[P<hir::Pat>]) {
        let tcx = self.bccx.tcx;
        let mut mutables = FxHashMap();
        for p in pats {
            p.each_binding(|_, hir_id, span, path1| {
                let name = path1.node;

                // Skip anything that looks like `_foo`
                if name.as_str().starts_with("_") {
                    return;
                }

                // Skip anything that looks like `&foo` or `&mut foo`, only look
                // for by-value bindings
                let bm = match self.bccx.tables.pat_binding_modes().get(hir_id) {
                    Some(&bm) => bm,
                    None => span_bug!(span, "missing binding mode"),
                };
                match bm {
                    ty::BindByValue(hir::MutMutable) => {}
                    _ => return,
                }

                mutables.entry(name).or_insert(Vec::new()).push((hir_id, span));
            });
        }

        for (_name, ids) in mutables {
            // If any id for this name was used mutably then consider them all
            // ok, so move on to the next
            if ids.iter().any(|&(ref hir_id, _)| self.used_mut.contains(hir_id)) {
                continue;
            }

            let (hir_id, span) = ids[0];
            let mut_span = tcx.sess.codemap().span_until_non_whitespace(span);

            // Ok, every name wasn't used mutably, so issue a warning that this
            // didn't need to be mutable.
            tcx.struct_span_lint_hir(UNUSED_MUT,
                                     hir_id,
                                     span,
                                     "variable does not need to be mutable")
                .span_suggestion_short(mut_span, "remove this `mut`", "".to_owned())
                .emit();
        }
    }
}

impl<'a, 'tcx> Visitor<'tcx> for UnusedMutCx<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::OnlyBodies(&self.bccx.tcx.hir)
    }

    fn visit_arm(&mut self, arm: &hir::Arm) {
        self.check_unused_mut_pat(&arm.pats)
    }

    fn visit_local(&mut self, local: &hir::Local) {
        self.check_unused_mut_pat(slice::from_ref(&local.pat));
    }
}

impl<'a, 'tcx> Visitor<'tcx> for UsedMutFinder<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::OnlyBodies(&self.bccx.tcx.hir)
    }

    fn visit_nested_body(&mut self, id: hir::BodyId) {
        let def_id = self.bccx.tcx.hir.body_owner_def_id(id);
        self.set.extend(self.bccx.tcx.borrowck(def_id).used_mut_nodes.iter().cloned());
        self.visit_body(self.bccx.tcx.hir.body(id));
    }
}
