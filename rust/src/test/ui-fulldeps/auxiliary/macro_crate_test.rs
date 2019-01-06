// force-host

#![feature(plugin_registrar, quote, rustc_private)]

extern crate syntax;
extern crate syntax_pos;
extern crate rustc;
extern crate rustc_plugin;

use syntax::ast::{self, Item, MetaItem, ItemKind};
use syntax::ext::base::*;
use syntax::parse;
use syntax::ptr::P;
use syntax::symbol::Symbol;
use syntax::tokenstream::TokenTree;
use syntax_pos::Span;
use rustc_plugin::Registry;

#[macro_export]
macro_rules! exported_macro { () => (2) }
macro_rules! unexported_macro { () => (3) }

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("make_a_1", expand_make_a_1);
    reg.register_macro("identity", expand_identity);
    reg.register_syntax_extension(
        Symbol::intern("into_multi_foo"),
        MultiModifier(Box::new(expand_into_foo_multi)));
    reg.register_syntax_extension(
        Symbol::intern("noop_attribute"),
        MultiModifier(Box::new(expand_noop_attribute)));
    reg.register_syntax_extension(
        Symbol::intern("duplicate"),
        MultiDecorator(Box::new(expand_duplicate)));
}

fn expand_make_a_1(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])
                   -> Box<MacResult+'static> {
    if !tts.is_empty() {
        cx.span_fatal(sp, "make_a_1 takes no arguments");
    }
    MacEager::expr(quote_expr!(cx, 1))
}

// See Issue #15750
fn expand_identity(cx: &mut ExtCtxt, _span: Span, tts: &[TokenTree])
                   -> Box<MacResult+'static> {
    // Parse an expression and emit it unchanged.
    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), tts.to_vec());
    let expr = parser.parse_expr().unwrap();
    MacEager::expr(quote_expr!(&mut *cx, $expr))
}

fn expand_into_foo_multi(cx: &mut ExtCtxt,
                         _sp: Span,
                         _attr: &MetaItem,
                         it: Annotatable) -> Annotatable {
    match it {
        Annotatable::Item(it) => {
            Annotatable::Item(P(Item {
                attrs: it.attrs.clone(),
                ..(*quote_item!(cx, enum Foo2 { Bar2, Baz2 }).unwrap()).clone()
            }))
        }
        Annotatable::ImplItem(_) => {
            quote_item!(cx, impl X { fn foo(&self) -> i32 { 42 } }).unwrap().and_then(|i| {
                match i.node {
                    ItemKind::Impl(.., mut items) => {
                        Annotatable::ImplItem(P(items.pop().expect("impl method not found")))
                    }
                    _ => unreachable!("impl parsed to something other than impl")
                }
            })
        }
        Annotatable::TraitItem(_) => {
            quote_item!(cx, trait X { fn foo(&self) -> i32 { 0 } }).unwrap().and_then(|i| {
                match i.node {
                    ItemKind::Trait(.., mut items) => {
                        Annotatable::TraitItem(P(items.pop().expect("trait method not found")))
                    }
                    _ => unreachable!("trait parsed to something other than trait")
                }
            })
        }
        // covered in proc_macro/macros-in-extern.rs
        Annotatable::ForeignItem(_) => unimplemented!(),
        // covered in proc_macro/attr-stmt-expr.rs
        Annotatable::Stmt(_) | Annotatable::Expr(_) => panic!("expected item")
    }
}

fn expand_noop_attribute(_cx: &mut ExtCtxt,
                         _sp: Span,
                         _attr: &MetaItem,
                         it: Annotatable) -> Annotatable {
    it
}

// Create a duplicate of the annotatable, based on the MetaItem
fn expand_duplicate(cx: &mut ExtCtxt,
                    _sp: Span,
                    mi: &MetaItem,
                    it: &Annotatable,
                    push: &mut FnMut(Annotatable))
{
    let copy_name = match mi.node {
        ast::MetaItemKind::List(ref xs) => {
            if let Some(word) = xs[0].word() {
                word.ident.segments.last().unwrap().ident
            } else {
                cx.span_err(mi.span, "Expected word");
                return;
            }
        }
        _ => {
            cx.span_err(mi.span, "Expected list");
            return;
        }
    };

    // Duplicate the item but replace its ident by the MetaItem
    match it.clone() {
        Annotatable::Item(it) => {
            let mut new_it = (*it).clone();
            new_it.attrs.clear();
            new_it.ident = copy_name;
            push(Annotatable::Item(P(new_it)));
        }
        Annotatable::ImplItem(it) => {
            let mut new_it = (*it).clone();
            new_it.attrs.clear();
            new_it.ident = copy_name;
            push(Annotatable::ImplItem(P(new_it)));
        }
        Annotatable::TraitItem(tt) => {
            let mut new_it = (*tt).clone();
            new_it.attrs.clear();
            new_it.ident = copy_name;
            push(Annotatable::TraitItem(P(new_it)));
        }
        // covered in proc_macro/macros-in-extern.rs
        Annotatable::ForeignItem(_) => unimplemented!(),
        // covered in proc_macro/attr-stmt-expr.rs
        Annotatable::Stmt(_) | Annotatable::Expr(_) => panic!("expected item")
    }
}

pub fn foo() {}
