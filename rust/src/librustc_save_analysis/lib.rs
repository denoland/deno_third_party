// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://doc.rust-lang.org/favicon.ico",
       html_root_url = "https://doc.rust-lang.org/nightly/")]
#![feature(custom_attribute)]
#![cfg_attr(stage0, feature(macro_lifetime_matcher))]
#![allow(unused_attributes)]

#![recursion_limit="256"]

#[macro_use]
extern crate rustc;

#[macro_use]
extern crate log;
extern crate rustc_data_structures;
extern crate rustc_serialize;
extern crate rustc_target;
extern crate rustc_typeck;
#[macro_use]
extern crate syntax;
extern crate syntax_pos;

extern crate rls_data;
extern crate rls_span;


mod json_dumper;
mod dump_visitor;
#[macro_use]
mod span_utils;
mod sig;

use rustc::hir;
use rustc::hir::def::Def as HirDef;
use rustc::hir::map::{Node, NodeTraitItem, NodeImplItem};
use rustc::hir::def_id::{DefId, LOCAL_CRATE};
use rustc::middle::cstore::ExternCrate;
use rustc::session::config::CrateType::CrateTypeExecutable;
use rustc::ty::{self, TyCtxt};
use rustc_typeck::hir_ty_to_ty;

use std::cell::Cell;
use std::default::Default;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};

use syntax::ast::{self, Attribute, NodeId, PatKind};
use syntax::parse::lexer::comments::strip_doc_comment_decoration;
use syntax::parse::token;
use syntax::print::pprust;
use syntax::symbol::keywords;
use syntax::visit::{self, Visitor};
use syntax::print::pprust::{arg_to_string, ty_to_string};
use syntax::codemap::MacroAttribute;
use syntax_pos::*;

use json_dumper::JsonDumper;
use dump_visitor::DumpVisitor;
use span_utils::SpanUtils;

use rls_data::{Def, DefKind, ExternalCrateData, GlobalCrateId, MacroRef, Ref, RefKind, Relation,
               RelationKind, SpanData, Impl, ImplKind};
use rls_data::config::Config;


pub struct SaveContext<'l, 'tcx: 'l> {
    tcx: TyCtxt<'l, 'tcx, 'tcx>,
    tables: &'l ty::TypeckTables<'tcx>,
    analysis: &'l ty::CrateAnalysis,
    span_utils: SpanUtils<'tcx>,
    config: Config,
    impl_counter: Cell<u32>,
}

#[derive(Debug)]
pub enum Data {
    RefData(Ref),
    DefData(Def),
    RelationData(Relation, Impl),
}

impl<'l, 'tcx: 'l> SaveContext<'l, 'tcx> {
    fn span_from_span(&self, span: Span) -> SpanData {
        use rls_span::{Column, Row};

        let cm = self.tcx.sess.codemap();
        let start = cm.lookup_char_pos(span.lo());
        let end = cm.lookup_char_pos(span.hi());

        SpanData {
            file_name: start.file.name.clone().to_string().into(),
            byte_start: span.lo().0,
            byte_end: span.hi().0,
            line_start: Row::new_one_indexed(start.line as u32),
            line_end: Row::new_one_indexed(end.line as u32),
            column_start: Column::new_one_indexed(start.col.0 as u32 + 1),
            column_end: Column::new_one_indexed(end.col.0 as u32 + 1),
        }
    }

    // List external crates used by the current crate.
    pub fn get_external_crates(&self) -> Vec<ExternalCrateData> {
        let mut result = Vec::new();

        for &n in self.tcx.crates().iter() {
            let span = match *self.tcx.extern_crate(n.as_def_id()) {
                Some(ExternCrate { span, .. }) => span,
                None => {
                    debug!("Skipping crate {}, no data", n);
                    continue;
                }
            };
            let lo_loc = self.span_utils.sess.codemap().lookup_char_pos(span.lo());
            result.push(ExternalCrateData {
                // FIXME: change file_name field to PathBuf in rls-data
                // https://github.com/nrc/rls-data/issues/7
                file_name: SpanUtils::make_path_string(&lo_loc.file.name),
                num: n.as_u32(),
                id: GlobalCrateId {
                    name: self.tcx.crate_name(n).to_string(),
                    disambiguator: self.tcx.crate_disambiguator(n).to_fingerprint().as_value(),
                },
            });
        }

        result
    }

    pub fn get_extern_item_data(&self, item: &ast::ForeignItem) -> Option<Data> {
        let qualname = format!("::{}", self.tcx.node_path_str(item.id));
        match item.node {
            ast::ForeignItemKind::Fn(ref decl, ref generics) => {
                let sub_span = self.span_utils
                    .sub_span_after_keyword(item.span, keywords::Fn);
                filter!(self.span_utils, sub_span, item.span, None);

                Some(Data::DefData(Def {
                    kind: DefKind::Function,
                    id: id_from_node_id(item.id, self),
                    span: self.span_from_span(sub_span.unwrap()),
                    name: item.ident.to_string(),
                    qualname,
                    value: make_signature(decl, generics),
                    parent: None,
                    children: vec![],
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::foreign_item_signature(item, self),
                    attributes: lower_attributes(item.attrs.clone(), self),
                }))
            }
            ast::ForeignItemKind::Static(ref ty, m) => {
                let keyword = if m { keywords::Mut } else { keywords::Static };
                let sub_span = self.span_utils.sub_span_after_keyword(item.span, keyword);
                filter!(self.span_utils, sub_span, item.span, None);

                let id = ::id_from_node_id(item.id, self);
                let span = self.span_from_span(sub_span.unwrap());

                Some(Data::DefData(Def {
                    kind: DefKind::Static,
                    id,
                    span,
                    name: item.ident.to_string(),
                    qualname,
                    value: ty_to_string(ty),
                    parent: None,
                    children: vec![],
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::foreign_item_signature(item, self),
                    attributes: lower_attributes(item.attrs.clone(), self),
                }))
            }
            // FIXME(plietar): needs a new DefKind in rls-data
            ast::ForeignItemKind::Ty => None,
            ast::ForeignItemKind::Macro(..) => None,
        }
    }

    pub fn get_item_data(&self, item: &ast::Item) -> Option<Data> {
        match item.node {
            ast::ItemKind::Fn(ref decl, .., ref generics, _) => {
                let qualname = format!("::{}", self.tcx.node_path_str(item.id));
                let sub_span = self.span_utils
                    .sub_span_after_keyword(item.span, keywords::Fn);
                filter!(self.span_utils, sub_span, item.span, None);
                Some(Data::DefData(Def {
                    kind: DefKind::Function,
                    id: id_from_node_id(item.id, self),
                    span: self.span_from_span(sub_span.unwrap()),
                    name: item.ident.to_string(),
                    qualname,
                    value: make_signature(decl, generics),
                    parent: None,
                    children: vec![],
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::item_signature(item, self),
                    attributes: lower_attributes(item.attrs.clone(), self),
                }))
            }
            ast::ItemKind::Static(ref typ, mt, _) => {
                let qualname = format!("::{}", self.tcx.node_path_str(item.id));

                let keyword = match mt {
                    ast::Mutability::Mutable => keywords::Mut,
                    ast::Mutability::Immutable => keywords::Static,
                };

                let sub_span = self.span_utils.sub_span_after_keyword(item.span, keyword);
                filter!(self.span_utils, sub_span, item.span, None);

                let id = id_from_node_id(item.id, self);
                let span = self.span_from_span(sub_span.unwrap());

                Some(Data::DefData(Def {
                    kind: DefKind::Static,
                    id,
                    span,
                    name: item.ident.to_string(),
                    qualname,
                    value: ty_to_string(&typ),
                    parent: None,
                    children: vec![],
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::item_signature(item, self),
                    attributes: lower_attributes(item.attrs.clone(), self),
                }))
            }
            ast::ItemKind::Const(ref typ, _) => {
                let qualname = format!("::{}", self.tcx.node_path_str(item.id));
                let sub_span = self.span_utils
                    .sub_span_after_keyword(item.span, keywords::Const);
                filter!(self.span_utils, sub_span, item.span, None);

                let id = id_from_node_id(item.id, self);
                let span = self.span_from_span(sub_span.unwrap());

                Some(Data::DefData(Def {
                    kind: DefKind::Const,
                    id,
                    span,
                    name: item.ident.to_string(),
                    qualname,
                    value: ty_to_string(typ),
                    parent: None,
                    children: vec![],
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::item_signature(item, self),
                    attributes: lower_attributes(item.attrs.clone(), self),
                }))
            }
            ast::ItemKind::Mod(ref m) => {
                let qualname = format!("::{}", self.tcx.node_path_str(item.id));

                let cm = self.tcx.sess.codemap();
                let filename = cm.span_to_filename(m.inner);

                let sub_span = self.span_utils
                    .sub_span_after_keyword(item.span, keywords::Mod);
                filter!(self.span_utils, sub_span, item.span, None);

                Some(Data::DefData(Def {
                    kind: DefKind::Mod,
                    id: id_from_node_id(item.id, self),
                    name: item.ident.to_string(),
                    qualname,
                    span: self.span_from_span(sub_span.unwrap()),
                    value: filename.to_string(),
                    parent: None,
                    children: m.items
                        .iter()
                        .map(|i| id_from_node_id(i.id, self))
                        .collect(),
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::item_signature(item, self),
                    attributes: lower_attributes(item.attrs.clone(), self),
                }))
            }
            ast::ItemKind::Enum(ref def, _) => {
                let name = item.ident.to_string();
                let qualname = format!("::{}", self.tcx.node_path_str(item.id));
                let sub_span = self.span_utils
                    .sub_span_after_keyword(item.span, keywords::Enum);
                filter!(self.span_utils, sub_span, item.span, None);
                let variants_str = def.variants
                    .iter()
                    .map(|v| v.node.ident.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let value = format!("{}::{{{}}}", name, variants_str);
                Some(Data::DefData(Def {
                    kind: DefKind::Enum,
                    id: id_from_node_id(item.id, self),
                    span: self.span_from_span(sub_span.unwrap()),
                    name,
                    qualname,
                    value,
                    parent: None,
                    children: def.variants
                        .iter()
                        .map(|v| id_from_node_id(v.node.data.id(), self))
                        .collect(),
                    decl_id: None,
                    docs: self.docs_for_attrs(&item.attrs),
                    sig: sig::item_signature(item, self),
                    attributes: lower_attributes(item.attrs.to_owned(), self),
                }))
            }
            ast::ItemKind::Impl(.., ref trait_ref, ref typ, ref impls) => {
                if let ast::TyKind::Path(None, ref path) = typ.node {
                    // Common case impl for a struct or something basic.
                    if generated_code(path.span) {
                        return None;
                    }
                    let sub_span = self.span_utils.sub_span_for_type_name(path.span);
                    filter!(self.span_utils, sub_span, typ.span, None);

                    let impl_id = self.next_impl_id();
                    let span = self.span_from_span(sub_span.unwrap());

                    let type_data = self.lookup_ref_id(typ.id);
                    type_data.map(|type_data| {
                        Data::RelationData(Relation {
                            kind: RelationKind::Impl {
                                id: impl_id,
                            },
                            span: span.clone(),
                            from: id_from_def_id(type_data),
                            to: trait_ref
                                .as_ref()
                                .and_then(|t| self.lookup_ref_id(t.ref_id))
                                .map(id_from_def_id)
                                .unwrap_or(null_id()),
                        },
                        Impl {
                            id: impl_id,
                            kind: match *trait_ref {
                                Some(_) => ImplKind::Direct,
                                None => ImplKind::Inherent,
                            },
                            span: span,
                            value: String::new(),
                            parent: None,
                            children: impls
                                .iter()
                                .map(|i| id_from_node_id(i.id, self))
                                .collect(),
                            docs: String::new(),
                            sig: None,
                            attributes: vec![],
                        })
                    })
                } else {
                    None
                }
            }
            _ => {
                // FIXME
                bug!();
            }
        }
    }

    pub fn get_field_data(&self, field: &ast::StructField, scope: NodeId) -> Option<Def> {
        if let Some(ident) = field.ident {
            let name = ident.to_string();
            let qualname = format!("::{}::{}", self.tcx.node_path_str(scope), ident);
            let sub_span = self.span_utils
                .sub_span_before_token(field.span, token::Colon);
            filter!(self.span_utils, sub_span, field.span, None);
            let def_id = self.tcx.hir.local_def_id(field.id);
            let typ = self.tcx.type_of(def_id).to_string();


            let id = id_from_node_id(field.id, self);
            let span = self.span_from_span(sub_span.unwrap());

            Some(Def {
                kind: DefKind::Field,
                id,
                span,
                name,
                qualname,
                value: typ,
                parent: Some(id_from_node_id(scope, self)),
                children: vec![],
                decl_id: None,
                docs: self.docs_for_attrs(&field.attrs),
                sig: sig::field_signature(field, self),
                attributes: lower_attributes(field.attrs.clone(), self),
            })
        } else {
            None
        }
    }

    // FIXME would be nice to take a MethodItem here, but the ast provides both
    // trait and impl flavours, so the caller must do the disassembly.
    pub fn get_method_data(&self, id: ast::NodeId, name: ast::Name, span: Span) -> Option<Def> {
        // The qualname for a method is the trait name or name of the struct in an impl in
        // which the method is declared in, followed by the method's name.
        let (qualname, parent_scope, decl_id, docs, attributes) =
            match self.tcx.impl_of_method(self.tcx.hir.local_def_id(id)) {
                Some(impl_id) => match self.tcx.hir.get_if_local(impl_id) {
                    Some(Node::NodeItem(item)) => match item.node {
                        hir::ItemImpl(.., ref ty, _) => {
                            let mut qualname = String::from("<");
                            qualname.push_str(&self.tcx.hir.node_to_pretty_string(ty.id));

                            let mut trait_id = self.tcx.trait_id_of_impl(impl_id);
                            let mut decl_id = None;
                            let mut docs = String::new();
                            let mut attrs = vec![];
                            if let Some(NodeImplItem(item)) = self.tcx.hir.find(id) {
                                docs = self.docs_for_attrs(&item.attrs);
                                attrs = item.attrs.to_vec();
                            }

                            if let Some(def_id) = trait_id {
                                // A method in a trait impl.
                                qualname.push_str(" as ");
                                qualname.push_str(&self.tcx.item_path_str(def_id));
                                self.tcx
                                    .associated_items(def_id)
                                    .find(|item| item.name == name)
                                    .map(|item| decl_id = Some(item.def_id));
                            }
                            qualname.push_str(">");

                            (qualname, trait_id, decl_id, docs, attrs)
                        }
                        _ => {
                            span_bug!(
                                span,
                                "Container {:?} for method {} not an impl?",
                                impl_id,
                                id
                            );
                        }
                    },
                    r => {
                        span_bug!(
                            span,
                            "Container {:?} for method {} is not a node item {:?}",
                            impl_id,
                            id,
                            r
                        );
                    }
                },
                None => match self.tcx.trait_of_item(self.tcx.hir.local_def_id(id)) {
                    Some(def_id) => {
                        let mut docs = String::new();
                        let mut attrs = vec![];

                        if let Some(NodeTraitItem(item)) = self.tcx.hir.find(id) {
                            docs = self.docs_for_attrs(&item.attrs);
                            attrs = item.attrs.to_vec();
                        }

                        (
                            format!("::{}", self.tcx.item_path_str(def_id)),
                            Some(def_id),
                            None,
                            docs,
                            attrs,
                        )
                    }
                    None => {
                        debug!("Could not find container for method {} at {:?}", id, span);
                        // This is not necessarily a bug, if there was a compilation error,
                        // the tables we need might not exist.
                        return None;
                    }
                },
            };

        let qualname = format!("{}::{}", qualname, name);

        let sub_span = self.span_utils.sub_span_after_keyword(span, keywords::Fn);
        filter!(self.span_utils, sub_span, span, None);

        Some(Def {
            kind: DefKind::Method,
            id: id_from_node_id(id, self),
            span: self.span_from_span(sub_span.unwrap()),
            name: name.to_string(),
            qualname,
            // FIXME you get better data here by using the visitor.
            value: String::new(),
            parent: parent_scope.map(|id| id_from_def_id(id)),
            children: vec![],
            decl_id: decl_id.map(|id| id_from_def_id(id)),
            docs,
            sig: None,
            attributes: lower_attributes(attributes, self),
        })
    }

    pub fn get_trait_ref_data(&self, trait_ref: &ast::TraitRef) -> Option<Ref> {
        self.lookup_ref_id(trait_ref.ref_id).and_then(|def_id| {
            let span = trait_ref.path.span;
            if generated_code(span) {
                return None;
            }
            let sub_span = self.span_utils.sub_span_for_type_name(span).or(Some(span));
            filter!(self.span_utils, sub_span, span, None);
            let span = self.span_from_span(sub_span.unwrap());
            Some(Ref {
                kind: RefKind::Type,
                span,
                ref_id: id_from_def_id(def_id),
            })
        })
    }

    pub fn get_expr_data(&self, expr: &ast::Expr) -> Option<Data> {
        let hir_node = self.tcx.hir.expect_expr(expr.id);
        let ty = self.tables.expr_ty_adjusted_opt(&hir_node);
        if ty.is_none() || ty.unwrap().sty == ty::TyError {
            return None;
        }
        match expr.node {
            ast::ExprKind::Field(ref sub_ex, ident) => {
                let hir_node = match self.tcx.hir.find(sub_ex.id) {
                    Some(Node::NodeExpr(expr)) => expr,
                    _ => {
                        debug!(
                            "Missing or weird node for sub-expression {} in {:?}",
                            sub_ex.id,
                            expr
                        );
                        return None;
                    }
                };
                match self.tables.expr_ty_adjusted(&hir_node).sty {
                    ty::TyAdt(def, _) if !def.is_enum() => {
                        let variant = &def.non_enum_variant();
                        let index = self.tcx.find_field_index(ident, variant).unwrap();
                        let sub_span = self.span_utils.span_for_last_ident(expr.span);
                        filter!(self.span_utils, sub_span, expr.span, None);
                        let span = self.span_from_span(sub_span.unwrap());
                        return Some(Data::RefData(Ref {
                            kind: RefKind::Variable,
                            span,
                            ref_id: id_from_def_id(variant.fields[index].did),
                        }));
                    }
                    ty::TyTuple(..) => None,
                    _ => {
                        debug!("Expected struct or union type, found {:?}", ty);
                        None
                    }
                }
            }
            ast::ExprKind::Struct(ref path, ..) => {
                match self.tables.expr_ty_adjusted(&hir_node).sty {
                    ty::TyAdt(def, _) if !def.is_enum() => {
                        let sub_span = self.span_utils.span_for_last_ident(path.span);
                        filter!(self.span_utils, sub_span, path.span, None);
                        let span = self.span_from_span(sub_span.unwrap());
                        Some(Data::RefData(Ref {
                            kind: RefKind::Type,
                            span,
                            ref_id: id_from_def_id(def.did),
                        }))
                    }
                    _ => {
                        // FIXME ty could legitimately be an enum, but then we will fail
                        // later if we try to look up the fields.
                        debug!("expected struct or union, found {:?}", ty);
                        None
                    }
                }
            }
            ast::ExprKind::MethodCall(ref seg, ..) => {
                let expr_hir_id = self.tcx.hir.definitions().node_to_hir_id(expr.id);
                let method_id = match self.tables.type_dependent_defs().get(expr_hir_id) {
                    Some(id) => id.def_id(),
                    None => {
                        debug!("Could not resolve method id for {:?}", expr);
                        return None;
                    }
                };
                let (def_id, decl_id) = match self.tcx.associated_item(method_id).container {
                    ty::ImplContainer(_) => (Some(method_id), None),
                    ty::TraitContainer(_) => (None, Some(method_id)),
                };
                let sub_span = seg.ident.span;
                filter!(self.span_utils, Some(sub_span), expr.span, None);
                let span = self.span_from_span(sub_span);
                Some(Data::RefData(Ref {
                    kind: RefKind::Function,
                    span,
                    ref_id: def_id
                        .or(decl_id)
                        .map(|id| id_from_def_id(id))
                        .unwrap_or(null_id()),
                }))
            }
            ast::ExprKind::Path(_, ref path) => {
                self.get_path_data(expr.id, path).map(|d| Data::RefData(d))
            }
            _ => {
                // FIXME
                bug!();
            }
        }
    }

    pub fn get_path_def(&self, id: NodeId) -> HirDef {
        match self.tcx.hir.get(id) {
            Node::NodeTraitRef(tr) => tr.path.def,

            Node::NodeItem(&hir::Item {
                node: hir::ItemUse(ref path, _),
                ..
            }) |
            Node::NodeVisibility(&hir::Visibility::Restricted { ref path, .. }) => path.def,

            Node::NodeExpr(&hir::Expr {
                node: hir::ExprStruct(ref qpath, ..),
                ..
            }) |
            Node::NodeExpr(&hir::Expr {
                node: hir::ExprPath(ref qpath),
                ..
            }) |
            Node::NodePat(&hir::Pat {
                node: hir::PatKind::Path(ref qpath),
                ..
            }) |
            Node::NodePat(&hir::Pat {
                node: hir::PatKind::Struct(ref qpath, ..),
                ..
            }) |
            Node::NodePat(&hir::Pat {
                node: hir::PatKind::TupleStruct(ref qpath, ..),
                ..
            }) => {
                let hir_id = self.tcx.hir.node_to_hir_id(id);
                self.tables.qpath_def(qpath, hir_id)
            }

            Node::NodeBinding(&hir::Pat {
                node: hir::PatKind::Binding(_, canonical_id, ..),
                ..
            }) => HirDef::Local(canonical_id),

            Node::NodeTy(ty) => if let hir::Ty {
                node: hir::TyPath(ref qpath),
                ..
            } = *ty
            {
                match *qpath {
                    hir::QPath::Resolved(_, ref path) => path.def,
                    hir::QPath::TypeRelative(..) => {
                        let ty = hir_ty_to_ty(self.tcx, ty);
                        if let ty::TyProjection(proj) = ty.sty {
                            return HirDef::AssociatedTy(proj.item_def_id);
                        }
                        HirDef::Err
                    }
                }
            } else {
                HirDef::Err
            },

            _ => HirDef::Err,
        }
    }

    pub fn get_path_data(&self, id: NodeId, path: &ast::Path) -> Option<Ref> {
        // Returns true if the path is function type sugar, e.g., `Fn(A) -> B`.
        fn fn_type(path: &ast::Path) -> bool {
            if path.segments.len() != 1 {
                return false;
            }
            if let Some(ref params) = path.segments[0].parameters {
                if let ast::PathParameters::Parenthesized(_) = **params {
                    return true;
                }
            }
            false
        }

        if path.segments.is_empty() {
            return None;
        }

        let def = self.get_path_def(id);
        let last_seg = &path.segments[path.segments.len() - 1];
        let sub_span = last_seg.ident.span;
        filter!(self.span_utils, Some(sub_span), path.span, None);
        match def {
            HirDef::Upvar(id, ..) | HirDef::Local(id) => {
                let span = self.span_from_span(sub_span);
                Some(Ref {
                    kind: RefKind::Variable,
                    span,
                    ref_id: id_from_node_id(id, self),
                })
            }
            HirDef::Static(..) |
            HirDef::Const(..) |
            HirDef::AssociatedConst(..) |
            HirDef::VariantCtor(..) => {
                let span = self.span_from_span(sub_span);
                Some(Ref {
                    kind: RefKind::Variable,
                    span,
                    ref_id: id_from_def_id(def.def_id()),
                })
            }
            HirDef::Trait(def_id) if fn_type(path) => {
                // Function type bounds are desugared in the parser, so we have to
                // special case them here.
                let fn_span = self.span_utils.span_for_first_ident(path.span);
                fn_span.map(|span| {
                    Ref {
                        kind: RefKind::Type,
                        span: self.span_from_span(span),
                        ref_id: id_from_def_id(def_id),
                    }
                })
            }
            HirDef::Struct(def_id) |
            HirDef::Variant(def_id, ..) |
            HirDef::Union(def_id) |
            HirDef::Enum(def_id) |
            HirDef::TyAlias(def_id) |
            HirDef::TyForeign(def_id) |
            HirDef::TraitAlias(def_id) |
            HirDef::AssociatedTy(def_id) |
            HirDef::Trait(def_id) |
            HirDef::Existential(def_id) |
            HirDef::TyParam(def_id) => {
                let span = self.span_from_span(sub_span);
                Some(Ref {
                    kind: RefKind::Type,
                    span,
                    ref_id: id_from_def_id(def_id),
                })
            }
            HirDef::StructCtor(def_id, _) => {
                // This is a reference to a tuple struct where the def_id points
                // to an invisible constructor function. That is not a very useful
                // def, so adjust to point to the tuple struct itself.
                let span = self.span_from_span(sub_span);
                let parent_def_id = self.tcx.parent_def_id(def_id).unwrap();
                Some(Ref {
                    kind: RefKind::Type,
                    span,
                    ref_id: id_from_def_id(parent_def_id),
                })
            }
            HirDef::Method(decl_id) => {
                let def_id = if decl_id.is_local() {
                    let ti = self.tcx.associated_item(decl_id);
                    self.tcx
                        .associated_items(ti.container.id())
                        .find(|item| item.name == ti.name && item.defaultness.has_value())
                        .map(|item| item.def_id)
                } else {
                    None
                };
                let span = self.span_from_span(sub_span);
                Some(Ref {
                    kind: RefKind::Function,
                    span,
                    ref_id: id_from_def_id(def_id.unwrap_or(decl_id)),
                })
            }
            HirDef::Fn(def_id) => {
                let span = self.span_from_span(sub_span);
                Some(Ref {
                    kind: RefKind::Function,
                    span,
                    ref_id: id_from_def_id(def_id),
                })
            }
            HirDef::Mod(def_id) => {
                let span = self.span_from_span(sub_span);
                Some(Ref {
                    kind: RefKind::Mod,
                    span,
                    ref_id: id_from_def_id(def_id),
                })
            }
            HirDef::PrimTy(..) |
            HirDef::SelfTy(..) |
            HirDef::Label(..) |
            HirDef::Macro(..) |
            HirDef::GlobalAsm(..) |
            HirDef::Err => None,
        }
    }

    pub fn get_field_ref_data(
        &self,
        field_ref: &ast::Field,
        variant: &ty::VariantDef,
    ) -> Option<Ref> {
        let index = self.tcx.find_field_index(field_ref.ident, variant).unwrap();
        // We don't really need a sub-span here, but no harm done
        let sub_span = self.span_utils.span_for_last_ident(field_ref.ident.span);
        filter!(self.span_utils, sub_span, field_ref.ident.span, None);
        let span = self.span_from_span(sub_span.unwrap());
        Some(Ref {
            kind: RefKind::Variable,
            span,
            ref_id: id_from_def_id(variant.fields[index].did),
        })
    }

    /// Attempt to return MacroRef for any AST node.
    ///
    /// For a given piece of AST defined by the supplied Span and NodeId,
    /// returns None if the node is not macro-generated or the span is malformed,
    /// else uses the expansion callsite and callee to return some MacroRef.
    pub fn get_macro_use_data(&self, span: Span) -> Option<MacroRef> {
        if !generated_code(span) {
            return None;
        }
        // Note we take care to use the source callsite/callee, to handle
        // nested expansions and ensure we only generate data for source-visible
        // macro uses.
        let callsite = span.source_callsite();
        let callsite_span = self.span_from_span(callsite);
        let callee = span.source_callee()?;
        let callee_span = callee.span?;

        // Ignore attribute macros, their spans are usually mangled
        if let MacroAttribute(_) = callee.format {
            return None;
        }

        // If the callee is an imported macro from an external crate, need to get
        // the source span and name from the session, as their spans are localized
        // when read in, and no longer correspond to the source.
        if let Some(mac) = self.tcx
            .sess
            .imported_macro_spans
            .borrow()
            .get(&callee_span)
        {
            let &(ref mac_name, mac_span) = mac;
            let mac_span = self.span_from_span(mac_span);
            return Some(MacroRef {
                span: callsite_span,
                qualname: mac_name.clone(), // FIXME: generate the real qualname
                callee_span: mac_span,
            });
        }

        let callee_span = self.span_from_span(callee_span);
        Some(MacroRef {
            span: callsite_span,
            qualname: callee.name().to_string(), // FIXME: generate the real qualname
            callee_span,
        })
    }

    fn lookup_ref_id(&self, ref_id: NodeId) -> Option<DefId> {
        match self.get_path_def(ref_id) {
            HirDef::PrimTy(_) | HirDef::SelfTy(..) | HirDef::Err => None,
            def => Some(def.def_id()),
        }
    }

    fn docs_for_attrs(&self, attrs: &[Attribute]) -> String {
        let mut result = String::new();

        for attr in attrs {
            if attr.check_name("doc") {
                if let Some(val) = attr.value_str() {
                    if attr.is_sugared_doc {
                        result.push_str(&strip_doc_comment_decoration(&val.as_str()));
                    } else {
                        result.push_str(&val.as_str());
                    }
                    result.push('\n');
                } else if let Some(meta_list) = attr.meta_item_list() {
                    meta_list.into_iter()
                             .filter(|it| it.check_name("include"))
                             .filter_map(|it| it.meta_item_list().map(|l| l.to_owned()))
                             .flat_map(|it| it)
                             .filter(|meta| meta.check_name("contents"))
                             .filter_map(|meta| meta.value_str())
                             .for_each(|val| {
                                 result.push_str(&val.as_str());
                                 result.push('\n');
                             });
                }
            }
        }

        if !self.config.full_docs {
            if let Some(index) = result.find("\n\n") {
                result.truncate(index);
            }
        }

        result
    }

    fn next_impl_id(&self) -> u32 {
        let next = self.impl_counter.get();
        self.impl_counter.set(next + 1);
        next
    }
}

fn make_signature(decl: &ast::FnDecl, generics: &ast::Generics) -> String {
    let mut sig = "fn ".to_owned();
    if !generics.params.is_empty() {
        sig.push('<');
        sig.push_str(&generics
            .params
            .iter()
            .map(|param| match *param {
                ast::GenericParam::Lifetime(ref l) => l.lifetime.ident.name.to_string(),
                ast::GenericParam::Type(ref t) => t.ident.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", "));
        sig.push_str("> ");
    }
    sig.push('(');
    sig.push_str(&decl.inputs
        .iter()
        .map(arg_to_string)
        .collect::<Vec<_>>()
        .join(", "));
    sig.push(')');
    match decl.output {
        ast::FunctionRetTy::Default(_) => sig.push_str(" -> ()"),
        ast::FunctionRetTy::Ty(ref t) => sig.push_str(&format!(" -> {}", ty_to_string(t))),
    }

    sig
}

// An AST visitor for collecting paths (e.g., the names of structs) and formal
// variables (idents) from patterns.
struct PathCollector<'l> {
    collected_paths: Vec<(NodeId, &'l ast::Path)>,
    collected_idents: Vec<(NodeId, ast::Ident, ast::Mutability)>,
}

impl<'l> PathCollector<'l> {
    fn new() -> PathCollector<'l> {
        PathCollector {
            collected_paths: vec![],
            collected_idents: vec![],
        }
    }
}

impl<'l, 'a: 'l> Visitor<'a> for PathCollector<'l> {
    fn visit_pat(&mut self, p: &'a ast::Pat) {
        match p.node {
            PatKind::Struct(ref path, ..) => {
                self.collected_paths.push((p.id, path));
            }
            PatKind::TupleStruct(ref path, ..) | PatKind::Path(_, ref path) => {
                self.collected_paths.push((p.id, path));
            }
            PatKind::Ident(bm, ident, _) => {
                debug!(
                    "PathCollector, visit ident in pat {}: {:?} {:?}",
                    ident,
                    p.span,
                    ident.span
                );
                let immut = match bm {
                    // Even if the ref is mut, you can't change the ref, only
                    // the data pointed at, so showing the initialising expression
                    // is still worthwhile.
                    ast::BindingMode::ByRef(_) => ast::Mutability::Immutable,
                    ast::BindingMode::ByValue(mt) => mt,
                };
                self.collected_idents
                    .push((p.id, ident, immut));
            }
            _ => {}
        }
        visit::walk_pat(self, p);
    }
}

/// Defines what to do with the results of saving the analysis.
pub trait SaveHandler {
    fn save<'l, 'tcx>(
        &mut self,
        save_ctxt: SaveContext<'l, 'tcx>,
        krate: &ast::Crate,
        cratename: &str,
    );
}

/// Dump the save-analysis results to a file.
pub struct DumpHandler<'a> {
    odir: Option<&'a Path>,
    cratename: String,
}

impl<'a> DumpHandler<'a> {
    pub fn new(odir: Option<&'a Path>, cratename: &str) -> DumpHandler<'a> {
        DumpHandler {
            odir,
            cratename: cratename.to_owned(),
        }
    }

    fn output_file(&self, ctx: &SaveContext) -> File {
        let sess = &ctx.tcx.sess;
        let file_name = match ctx.config.output_file {
            Some(ref s) => PathBuf::from(s),
            None => {
                let mut root_path = match self.odir {
                    Some(val) => val.join("save-analysis"),
                    None => PathBuf::from("save-analysis-temp"),
                };

                if let Err(e) = std::fs::create_dir_all(&root_path) {
                    error!("Could not create directory {}: {}", root_path.display(), e);
                }

                let executable = sess.crate_types
                    .borrow()
                    .iter()
                    .any(|ct| *ct == CrateTypeExecutable);
                let mut out_name = if executable {
                    "".to_owned()
                } else {
                    "lib".to_owned()
                };
                out_name.push_str(&self.cratename);
                out_name.push_str(&sess.opts.cg.extra_filename);
                out_name.push_str(".json");
                root_path.push(&out_name);

                root_path
            }
        };

        info!("Writing output to {}", file_name.display());

        let output_file = File::create(&file_name).unwrap_or_else(
            |e| sess.fatal(&format!("Could not open {}: {}", file_name.display(), e)),
        );

        output_file
    }
}

impl<'a> SaveHandler for DumpHandler<'a> {
    fn save<'l, 'tcx>(
        &mut self,
        save_ctxt: SaveContext<'l, 'tcx>,
        krate: &ast::Crate,
        cratename: &str,
    ) {
        let output = &mut self.output_file(&save_ctxt);
        let mut dumper = JsonDumper::new(output, save_ctxt.config.clone());
        let mut visitor = DumpVisitor::new(save_ctxt, &mut dumper);

        visitor.dump_crate_info(cratename, krate);
        visit::walk_crate(&mut visitor, krate);
    }
}

/// Call a callback with the results of save-analysis.
pub struct CallbackHandler<'b> {
    pub callback: &'b mut FnMut(&rls_data::Analysis),
}

impl<'b> SaveHandler for CallbackHandler<'b> {
    fn save<'l, 'tcx>(
        &mut self,
        save_ctxt: SaveContext<'l, 'tcx>,
        krate: &ast::Crate,
        cratename: &str,
    ) {
        // We're using the JsonDumper here because it has the format of the
        // save-analysis results that we will pass to the callback. IOW, we are
        // using the JsonDumper to collect the save-analysis results, but not
        // actually to dump them to a file. This is all a bit convoluted and
        // there is certainly a simpler design here trying to get out (FIXME).
        let mut dumper = JsonDumper::with_callback(self.callback, save_ctxt.config.clone());
        let mut visitor = DumpVisitor::new(save_ctxt, &mut dumper);

        visitor.dump_crate_info(cratename, krate);
        visit::walk_crate(&mut visitor, krate);
    }
}

pub fn process_crate<'l, 'tcx, H: SaveHandler>(
    tcx: TyCtxt<'l, 'tcx, 'tcx>,
    krate: &ast::Crate,
    analysis: &'l ty::CrateAnalysis,
    cratename: &str,
    config: Option<Config>,
    mut handler: H,
) {
    tcx.dep_graph.with_ignore(|| {
        assert!(analysis.glob_map.is_some());

        info!("Dumping crate {}", cratename);

        let save_ctxt = SaveContext {
            tcx,
            tables: &ty::TypeckTables::empty(None),
            analysis,
            span_utils: SpanUtils::new(&tcx.sess),
            config: find_config(config),
            impl_counter: Cell::new(0),
        };

        handler.save(save_ctxt, krate, cratename)
    })
}

fn find_config(supplied: Option<Config>) -> Config {
    if let Some(config) = supplied {
        return config;
    }
    match env::var_os("RUST_SAVE_ANALYSIS_CONFIG") {
        Some(config_string) => rustc_serialize::json::decode(config_string.to_str().unwrap())
            .expect("Could not deserialize save-analysis config"),
        None => Config::default(),
    }
}

// Utility functions for the module.

// Helper function to escape quotes in a string
fn escape(s: String) -> String {
    s.replace("\"", "\"\"")
}

// Helper function to determine if a span came from a
// macro expansion or syntax extension.
fn generated_code(span: Span) -> bool {
    span.ctxt() != NO_EXPANSION || span == DUMMY_SP
}

// DefId::index is a newtype and so the JSON serialisation is ugly. Therefore
// we use our own Id which is the same, but without the newtype.
fn id_from_def_id(id: DefId) -> rls_data::Id {
    rls_data::Id {
        krate: id.krate.as_u32(),
        index: id.index.as_raw_u32(),
    }
}

fn id_from_node_id(id: NodeId, scx: &SaveContext) -> rls_data::Id {
    let def_id = scx.tcx.hir.opt_local_def_id(id);
    def_id.map(|id| id_from_def_id(id)).unwrap_or_else(|| {
        // Create a *fake* `DefId` out of a `NodeId` by subtracting the `NodeId`
        // out of the maximum u32 value. This will work unless you have *billions*
        // of definitions in a single crate (very unlikely to actually happen).
        rls_data::Id {
            krate: LOCAL_CRATE.as_u32(),
            index: !id.as_u32(),
        }
    })
}

fn null_id() -> rls_data::Id {
    rls_data::Id {
        krate: u32::max_value(),
        index: u32::max_value(),
    }
}

fn lower_attributes(attrs: Vec<Attribute>, scx: &SaveContext) -> Vec<rls_data::Attribute> {
    attrs.into_iter()
    // Only retain real attributes. Doc comments are lowered separately.
    .filter(|attr| attr.path != "doc")
    .map(|mut attr| {
        // Remove the surrounding '#[..]' or '#![..]' of the pretty printed
        // attribute. First normalize all inner attribute (#![..]) to outer
        // ones (#[..]), then remove the two leading and the one trailing character.
        attr.style = ast::AttrStyle::Outer;
        let value = pprust::attribute_to_string(&attr);
        // This str slicing works correctly, because the leading and trailing characters
        // are in the ASCII range and thus exactly one byte each.
        let value = value[2..value.len()-1].to_string();

        rls_data::Attribute {
            value,
            span: scx.span_from_span(attr.span),
        }
    }).collect()
}
