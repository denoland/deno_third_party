// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// force-host

#![feature(plugin_registrar)]
#![feature(box_syntax)]
#![feature(rustc_private)]

extern crate syntax;
extern crate syntax_ext;
extern crate syntax_pos;
extern crate rustc;
extern crate rustc_plugin;

use syntax::ast;
use syntax::attr;
use syntax::ext::base::{MultiDecorator, ExtCtxt, Annotatable};
use syntax::ext::build::AstBuilder;
use syntax::symbol::Symbol;
use syntax::ptr::P;
use syntax_ext::deriving::generic::{TraitDef, MethodDef, combine_substructure};
use syntax_ext::deriving::generic::{Substructure, Struct, EnumMatching};
use syntax_ext::deriving::generic::ty::{Literal, LifetimeBounds, Path, borrowed_explicit_self};
use syntax_pos::Span;
use rustc_plugin::Registry;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(
        Symbol::intern("derive_TotalSum"),
        MultiDecorator(box expand));
}

fn expand(cx: &mut ExtCtxt,
          span: Span,
          mitem: &ast::MetaItem,
          item: &Annotatable,
          push: &mut FnMut(Annotatable)) {
    let trait_def = TraitDef {
        span: span,
        attributes: vec![],
        path: Path::new_local("TotalSum"),
        additional_bounds: vec![],
        generics: LifetimeBounds::empty(),
        associated_types: vec![],
        is_unsafe: false,
        supports_unions: false,
        methods: vec![
            MethodDef {
                name: "total_sum",
                generics: LifetimeBounds::empty(),
                explicit_self: borrowed_explicit_self(),
                args: vec![],
                ret_ty: Literal(Path::new_local("isize")),
                attributes: vec![],
                is_unsafe: false,
                unify_fieldless_variants: true,
                combine_substructure: combine_substructure(Box::new(totalsum_substructure)),
            },
        ],
    };

    trait_def.expand(cx, mitem, item, push)
}

// Mostly copied from syntax::ext::deriving::hash
/// Defines how the implementation for `trace()` is to be generated
fn totalsum_substructure(cx: &mut ExtCtxt, trait_span: Span,
                         substr: &Substructure) -> P<ast::Expr> {
    let fields = match *substr.fields {
        Struct(_, ref fs) | EnumMatching(.., ref fs) => fs,
        _ => cx.span_bug(trait_span, "impossible substructure")
    };

    fields.iter().fold(cx.expr_isize(trait_span, 0), |acc, ref item| {
        if attr::contains_name(&item.attrs, "ignore") {
            acc
        } else {
            cx.expr_binary(item.span, ast::BinOpKind::Add, acc,
                           cx.expr_method_call(item.span,
                                               item.self_.clone(),
                                               substr.method_ident,
                                               Vec::new()))
        }
    })
}
