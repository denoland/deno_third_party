// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use hir::def::Def;
use hir::def_id::DefId;
use ty::{self, Ty, TyCtxt};
use ty::layout::{LayoutError, Pointer, SizeSkeleton};

use rustc_target::spec::abi::Abi::RustIntrinsic;
use syntax_pos::Span;
use hir::intravisit::{self, Visitor, NestedVisitorMap};
use hir;

pub fn check_crate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>) {
    let mut visitor = ItemVisitor {
        tcx,
    };
    tcx.hir.krate().visit_all_item_likes(&mut visitor.as_deep_visitor());
}

struct ItemVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>
}

struct ExprVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    tables: &'tcx ty::TypeckTables<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
}

/// If the type is `Option<T>`, it will return `T`, otherwise
/// the type itself. Works on most `Option`-like types.
fn unpack_option_like<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                ty: Ty<'tcx>)
                                -> Ty<'tcx> {
    let (def, substs) = match ty.sty {
        ty::TyAdt(def, substs) => (def, substs),
        _ => return ty
    };

    if def.variants.len() == 2 && !def.repr.c() && def.repr.int.is_none() {
        let data_idx;

        if def.variants[0].fields.is_empty() {
            data_idx = 1;
        } else if def.variants[1].fields.is_empty() {
            data_idx = 0;
        } else {
            return ty;
        }

        if def.variants[data_idx].fields.len() == 1 {
            return def.variants[data_idx].fields[0].ty(tcx, substs);
        }
    }

    ty
}

impl<'a, 'tcx> ExprVisitor<'a, 'tcx> {
    fn def_id_is_transmute(&self, def_id: DefId) -> bool {
        self.tcx.fn_sig(def_id).abi() == RustIntrinsic &&
        self.tcx.item_name(def_id) == "transmute"
    }

    fn check_transmute(&self, span: Span, from: Ty<'tcx>, to: Ty<'tcx>) {
        let sk_from = SizeSkeleton::compute(from, self.tcx, self.param_env);
        let sk_to = SizeSkeleton::compute(to, self.tcx, self.param_env);

        // Check for same size using the skeletons.
        if let (Ok(sk_from), Ok(sk_to)) = (sk_from, sk_to) {
            if sk_from.same_size(sk_to) {
                return;
            }

            // Special-case transmutting from `typeof(function)` and
            // `Option<typeof(function)>` to present a clearer error.
            let from = unpack_option_like(self.tcx.global_tcx(), from);
            if let (&ty::TyFnDef(..), SizeSkeleton::Known(size_to)) = (&from.sty, sk_to) {
                if size_to == Pointer.size(self.tcx) {
                    struct_span_err!(self.tcx.sess, span, E0591,
                                     "can't transmute zero-sized type")
                        .note(&format!("source type: {}", from))
                        .note(&format!("target type: {}", to))
                        .help("cast with `as` to a pointer instead")
                        .emit();
                    return;
                }
            }
        }

        // Try to display a sensible error with as much information as possible.
        let skeleton_string = |ty: Ty<'tcx>, sk| {
            match sk {
                Ok(SizeSkeleton::Known(size)) => {
                    format!("{} bits", size.bits())
                }
                Ok(SizeSkeleton::Pointer { tail, .. }) => {
                    format!("pointer to {}", tail)
                }
                Err(LayoutError::Unknown(bad)) => {
                    if bad == ty {
                        format!("this type's size can vary")
                    } else {
                        format!("size can vary because of {}", bad)
                    }
                }
                Err(err) => err.to_string()
            }
        };

        struct_span_err!(self.tcx.sess, span, E0512,
            "transmute called with types of different sizes")
            .note(&format!("source type: {} ({})", from, skeleton_string(from, sk_from)))
            .note(&format!("target type: {} ({})", to, skeleton_string(to, sk_to)))
            .emit();
    }
}

impl<'a, 'tcx> Visitor<'tcx> for ItemVisitor<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }

    fn visit_nested_body(&mut self, body_id: hir::BodyId) {
        let owner_def_id = self.tcx.hir.body_owner_def_id(body_id);
        let body = self.tcx.hir.body(body_id);
        let param_env = self.tcx.param_env(owner_def_id);
        let tables = self.tcx.typeck_tables_of(owner_def_id);
        ExprVisitor { tcx: self.tcx, param_env, tables }.visit_body(body);
        self.visit_body(body);
    }
}

impl<'a, 'tcx> Visitor<'tcx> for ExprVisitor<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr) {
        let def = if let hir::ExprPath(ref qpath) = expr.node {
            self.tables.qpath_def(qpath, expr.hir_id)
        } else {
            Def::Err
        };
        if let Def::Fn(did) = def {
            if self.def_id_is_transmute(did) {
                let typ = self.tables.node_id_to_type(expr.hir_id);
                let sig = typ.fn_sig(self.tcx);
                let from = sig.inputs().skip_binder()[0];
                let to = *sig.output().skip_binder();
                self.check_transmute(expr.span, from, to);
            }
        }

        intravisit::walk_expr(self, expr);
    }
}
