// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Error Reporting for Anonymous Region Lifetime Errors
//! where both the regions are anonymous.

use infer::error_reporting::nice_region_error::NiceRegionError;
use infer::SubregionOrigin;
use ty::RegionKind;
use hir::{Expr, ExprClosure};
use hir::map::NodeExpr;
use util::common::ErrorReported;
use infer::lexical_region_resolve::RegionResolutionError::SubSupConflict;

impl<'a, 'gcx, 'tcx> NiceRegionError<'a, 'gcx, 'tcx> {
    /// Print the error message for lifetime errors when binding excapes a closure.
    ///
    /// Consider a case where we have
    ///
    /// ```no_run
    /// fn with_int<F>(f: F) where F: FnOnce(&isize) {
    ///     let x = 3;
    ///     f(&x);
    /// }
    /// fn main() {
    ///     let mut x = None;
    ///     with_int(|y| x = Some(y));
    /// }
    /// ```
    ///
    /// the output will be
    ///
    /// ```text
    ///     let mut x = None;
    ///         ----- borrowed data cannot be stored into here...
    ///     with_int(|y| x = Some(y));
    ///              ---          ^ cannot be stored outside of its closure
    ///              |
    ///              ...because it cannot outlive this closure
    /// ```
    pub(super) fn try_report_outlives_closure(&self) -> Option<ErrorReported> {
        if let Some(SubSupConflict(origin,
                                   ref sub_origin,
                                   _,
                                   ref sup_origin,
                                   sup_region)) = self.error {

            // #45983: when trying to assign the contents of an argument to a binding outside of a
            // closure, provide a specific message pointing this out.
            if let (&SubregionOrigin::BindingTypeIsNotValidAtDecl(ref external_span),
                    &RegionKind::ReFree(ref free_region)) = (&sub_origin, sup_region) {
                let hir = &self.tcx.hir;
                if let Some(node_id) = hir.as_local_node_id(free_region.scope) {
                    match hir.get(node_id) {
                        NodeExpr(Expr {
                            node: ExprClosure(_, _, _, closure_span, None),
                            ..
                        }) => {
                            let sup_sp = sup_origin.span();
                            let origin_sp = origin.span();
                            let mut err = self.tcx.sess.struct_span_err(
                                sup_sp,
                                "borrowed data cannot be stored outside of its closure");
                            err.span_label(sup_sp, "cannot be stored outside of its closure");
                            if origin_sp == sup_sp || origin_sp.contains(sup_sp) {
// // sup_sp == origin.span():
//
// let mut x = None;
//     ----- borrowed data cannot be stored into here...
// with_int(|y| x = Some(y));
//          ---          ^ cannot be stored outside of its closure
//          |
//          ...because it cannot outlive this closure
//
// // origin.contains(&sup_sp):
//
// let mut f: Option<&u32> = None;
//     ----- borrowed data cannot be stored into here...
// closure_expecting_bound(|x: &'x u32| {
//                         ------------ ... because it cannot outlive this closure
//     f = Some(x);
//              ^ cannot be stored outside of its closure
                                err.span_label(*external_span,
                                               "borrowed data cannot be stored into here...");
                                err.span_label(*closure_span,
                                               "...because it cannot outlive this closure");
                            } else {
// FIXME: the wording for this case could be much improved
//
// let mut lines_to_use: Vec<&CrateId> = Vec::new();
//                           - cannot infer an appropriate lifetime...
// let push_id = |installed_id: &CrateId| {
//     -------   ------------------------ borrowed data cannot outlive this closure
//     |
//     ...so that variable is valid at time of its declaration
//     lines_to_use.push(installed_id);
//                       ^^^^^^^^^^^^ cannot be stored outside of its closure
                                err.span_label(origin_sp,
                                               "cannot infer an appropriate lifetime...");
                                err.span_label(*external_span,
                                               "...so that variable is valid at time of its \
                                                declaration");
                                err.span_label(*closure_span,
                                               "borrowed data cannot outlive this closure");
                            }
                            err.emit();
                            return Some(ErrorReported);
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
}

