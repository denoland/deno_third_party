use std::mem;

use errors;

use syntax::ast::{self, Ident};
use syntax::attr;
use syntax::source_map::{ExpnInfo, MacroAttribute, hygiene, respan};
use syntax::ext::base::ExtCtxt;
use syntax::ext::build::AstBuilder;
use syntax::ext::expand::ExpansionConfig;
use syntax::ext::hygiene::Mark;
use syntax::fold::Folder;
use syntax::parse::ParseSess;
use syntax::ptr::P;
use syntax::symbol::Symbol;
use syntax::symbol::keywords;
use syntax::visit::{self, Visitor};

use syntax_pos::{Span, DUMMY_SP};

use deriving;

const PROC_MACRO_KINDS: [&str; 3] = ["proc_macro_derive", "proc_macro_attribute", "proc_macro"];

struct ProcMacroDerive {
    trait_name: ast::Name,
    function_name: Ident,
    span: Span,
    attrs: Vec<ast::Name>,
}

struct ProcMacroDef {
    function_name: Ident,
    span: Span,
}

struct CollectProcMacros<'a> {
    derives: Vec<ProcMacroDerive>,
    attr_macros: Vec<ProcMacroDef>,
    bang_macros: Vec<ProcMacroDef>,
    in_root: bool,
    handler: &'a errors::Handler,
    is_proc_macro_crate: bool,
    is_test_crate: bool,
}

pub fn modify(sess: &ParseSess,
              resolver: &mut dyn (::syntax::ext::base::Resolver),
              mut krate: ast::Crate,
              is_proc_macro_crate: bool,
              is_test_crate: bool,
              num_crate_types: usize,
              handler: &errors::Handler) -> ast::Crate {
    let ecfg = ExpansionConfig::default("proc_macro".to_string());
    let mut cx = ExtCtxt::new(sess, ecfg, resolver);

    let (derives, attr_macros, bang_macros) = {
        let mut collect = CollectProcMacros {
            derives: Vec::new(),
            attr_macros: Vec::new(),
            bang_macros: Vec::new(),
            in_root: true,
            handler,
            is_proc_macro_crate,
            is_test_crate,
        };
        visit::walk_crate(&mut collect, &krate);
        (collect.derives, collect.attr_macros, collect.bang_macros)
    };

    if !is_proc_macro_crate {
        return krate
    }

    if num_crate_types > 1 {
        handler.err("cannot mix `proc-macro` crate type with others");
    }

    if is_test_crate {
        return krate;
    }

    krate.module.items.push(mk_decls(&mut cx, &derives, &attr_macros, &bang_macros));

    krate
}

fn is_proc_macro_attr(attr: &ast::Attribute) -> bool {
    PROC_MACRO_KINDS.iter().any(|kind| attr.check_name(kind))
}

impl<'a> CollectProcMacros<'a> {
    fn check_not_pub_in_root(&self, vis: &ast::Visibility, sp: Span) {
        if self.is_proc_macro_crate && self.in_root && vis.node.is_pub() {
            self.handler.span_err(sp,
                                  "`proc-macro` crate types cannot \
                                   export any items other than functions \
                                   tagged with `#[proc_macro_derive]` currently");
        }
    }

    fn collect_custom_derive(&mut self, item: &'a ast::Item, attr: &'a ast::Attribute) {
        // Once we've located the `#[proc_macro_derive]` attribute, verify
        // that it's of the form `#[proc_macro_derive(Foo)]` or
        // `#[proc_macro_derive(Foo, attributes(A, ..))]`
        let list = match attr.meta_item_list() {
            Some(list) => list,
            None => {
                self.handler.span_err(attr.span(),
                                      "attribute must be of form: \
                                       #[proc_macro_derive(TraitName)]");
                return
            }
        };
        if list.len() != 1 && list.len() != 2 {
            self.handler.span_err(attr.span(),
                                  "attribute must have either one or two arguments");
            return
        }
        let trait_attr = &list[0];
        let attributes_attr = list.get(1);
        let trait_name = match trait_attr.name() {
            Some(name) => name,
            _ => {
                self.handler.span_err(trait_attr.span(), "not a meta item");
                return
            }
        };
        if !trait_attr.is_word() {
            self.handler.span_err(trait_attr.span(), "must only be one word");
        }

        if deriving::is_builtin_trait(trait_name) {
            self.handler.span_err(trait_attr.span(),
                                  "cannot override a built-in #[derive] mode");
        }

        let proc_attrs: Vec<_> = if let Some(attr) = attributes_attr {
            if !attr.check_name("attributes") {
                self.handler.span_err(attr.span(), "second argument must be `attributes`")
            }
            attr.meta_item_list().unwrap_or_else(|| {
                self.handler.span_err(attr.span(),
                                      "attribute must be of form: \
                                       `attributes(foo, bar)`");
                &[]
            }).into_iter().filter_map(|attr| {
                let name = match attr.name() {
                    Some(name) => name,
                    _ => {
                        self.handler.span_err(attr.span(), "not a meta item");
                        return None;
                    },
                };

                if !attr.is_word() {
                    self.handler.span_err(attr.span(), "must only be one word");
                    return None;
                }

                Some(name)
            }).collect()
        } else {
            Vec::new()
        };

        if self.in_root && item.vis.node.is_pub() {
            self.derives.push(ProcMacroDerive {
                span: item.span,
                trait_name,
                function_name: item.ident,
                attrs: proc_attrs,
            });
        } else {
            let msg = if !self.in_root {
                "functions tagged with `#[proc_macro_derive]` must \
                 currently reside in the root of the crate"
            } else {
                "functions tagged with `#[proc_macro_derive]` must be `pub`"
            };
            self.handler.span_err(item.span, msg);
        }
    }

    fn collect_attr_proc_macro(&mut self, item: &'a ast::Item, attr: &'a ast::Attribute) {
        if !attr.is_word() {
            self.handler.span_err(attr.span, "`#[proc_macro_attribute]` attribute \
                does not take any arguments");
            return;
        }

        if self.in_root && item.vis.node.is_pub() {
            self.attr_macros.push(ProcMacroDef {
                span: item.span,
                function_name: item.ident,
            });
        } else {
            let msg = if !self.in_root {
                "functions tagged with `#[proc_macro_attribute]` must \
                 currently reside in the root of the crate"
            } else {
                "functions tagged with `#[proc_macro_attribute]` must be `pub`"
            };
            self.handler.span_err(item.span, msg);
        }
    }

    fn collect_bang_proc_macro(&mut self, item: &'a ast::Item, attr: &'a ast::Attribute) {
        if !attr.is_word() {
            self.handler.span_err(attr.span, "`#[proc_macro]` attribute \
                does not take any arguments");
            return;
        }

        if self.in_root && item.vis.node.is_pub() {
            self.bang_macros.push(ProcMacroDef {
                span: item.span,
                function_name: item.ident,
            });
        } else {
            let msg = if !self.in_root {
                "functions tagged with `#[proc_macro]` must \
                 currently reside in the root of the crate"
            } else {
                "functions tagged with `#[proc_macro]` must be `pub`"
            };
            self.handler.span_err(item.span, msg);
        }
    }
}

impl<'a> Visitor<'a> for CollectProcMacros<'a> {
    fn visit_item(&mut self, item: &'a ast::Item) {
        if let ast::ItemKind::MacroDef(..) = item.node {
            if self.is_proc_macro_crate && attr::contains_name(&item.attrs, "macro_export") {
                let msg =
                    "cannot export macro_rules! macros from a `proc-macro` crate type currently";
                self.handler.span_err(item.span, msg);
            }
        }

        // First up, make sure we're checking a bare function. If we're not then
        // we're just not interested in this item.
        //
        // If we find one, try to locate a `#[proc_macro_derive]` attribute on
        // it.
        let is_fn = match item.node {
            ast::ItemKind::Fn(..) => true,
            _ => false,
        };

        let mut found_attr: Option<&'a ast::Attribute> = None;

        for attr in &item.attrs {
            if is_proc_macro_attr(&attr) {
                if let Some(prev_attr) = found_attr {
                    let msg = if attr.path.segments[0].ident.name ==
                                 prev_attr.path.segments[0].ident.name {
                        format!("Only one `#[{}]` attribute is allowed on any given function",
                                attr.path)
                    } else {
                        format!("`#[{}]` and `#[{}]` attributes cannot both be applied \
                                to the same function", attr.path, prev_attr.path)
                    };

                    self.handler.struct_span_err(attr.span(), &msg)
                        .span_note(prev_attr.span(), "Previous attribute here")
                        .emit();

                    return;
                }

                found_attr = Some(attr);
            }
        }

        let attr = match found_attr {
            None => {
                self.check_not_pub_in_root(&item.vis, item.span);
                let prev_in_root = mem::replace(&mut self.in_root, false);
                visit::walk_item(self, item);
                self.in_root = prev_in_root;
                return;
            },
            Some(attr) => attr,
        };

        if !is_fn {
            let msg = format!("the `#[{}]` attribute may only be used on bare functions",
                              attr.path);

            self.handler.span_err(attr.span(), &msg);
            return;
        }

        if self.is_test_crate {
            return;
        }

        if !self.is_proc_macro_crate {
            let msg = format!("the `#[{}]` attribute is only usable with crates of the \
                              `proc-macro` crate type", attr.path);

            self.handler.span_err(attr.span(), &msg);
            return;
        }

        if attr.check_name("proc_macro_derive") {
            self.collect_custom_derive(item, attr);
        } else if attr.check_name("proc_macro_attribute") {
            self.collect_attr_proc_macro(item, attr);
        } else if attr.check_name("proc_macro") {
            self.collect_bang_proc_macro(item, attr);
        };

        let prev_in_root = mem::replace(&mut self.in_root, false);
        visit::walk_item(self, item);
        self.in_root = prev_in_root;
    }

    fn visit_mac(&mut self, mac: &ast::Mac) {
        visit::walk_mac(self, mac)
    }
}

// Creates a new module which looks like:
//
//      mod $gensym {
//          extern crate proc_macro;
//
//          use proc_macro::bridge::client::ProcMacro;
//
//          #[rustc_proc_macro_decls]
//          static DECLS: &[ProcMacro] = &[
//              ProcMacro::custom_derive($name_trait1, &[], ::$name1);
//              ProcMacro::custom_derive($name_trait2, &["attribute_name"], ::$name2);
//              // ...
//          ];
//      }
fn mk_decls(
    cx: &mut ExtCtxt,
    custom_derives: &[ProcMacroDerive],
    custom_attrs: &[ProcMacroDef],
    custom_macros: &[ProcMacroDef],
) -> P<ast::Item> {
    let mark = Mark::fresh(Mark::root());
    mark.set_expn_info(ExpnInfo {
        call_site: DUMMY_SP,
        def_site: None,
        format: MacroAttribute(Symbol::intern("proc_macro")),
        allow_internal_unstable: true,
        allow_internal_unsafe: false,
        local_inner_macros: false,
        edition: hygiene::default_edition(),
    });
    let span = DUMMY_SP.apply_mark(mark);

    let proc_macro = Ident::from_str("proc_macro");
    let krate = cx.item(span,
                        proc_macro,
                        Vec::new(),
                        ast::ItemKind::ExternCrate(None));

    let bridge = Ident::from_str("bridge");
    let client = Ident::from_str("client");
    let proc_macro_ty = Ident::from_str("ProcMacro");
    let custom_derive = Ident::from_str("custom_derive");
    let attr = Ident::from_str("attr");
    let bang = Ident::from_str("bang");
    let crate_kw = Ident::with_empty_ctxt(keywords::Crate.name());

    let decls = {
        let local_path = |sp: Span, name| {
            cx.expr_path(cx.path(sp.with_ctxt(span.ctxt()), vec![crate_kw, name]))
        };
        let proc_macro_ty_method_path = |method| cx.expr_path(cx.path(span, vec![
            proc_macro, bridge, client, proc_macro_ty, method,
        ]));
        custom_derives.iter().map(|cd| {
            cx.expr_call(span, proc_macro_ty_method_path(custom_derive), vec![
                cx.expr_str(cd.span, cd.trait_name),
                cx.expr_vec_slice(
                    span,
                    cd.attrs.iter().map(|&s| cx.expr_str(cd.span, s)).collect::<Vec<_>>()
                ),
                local_path(cd.span, cd.function_name),
            ])
        }).chain(custom_attrs.iter().map(|ca| {
            cx.expr_call(span, proc_macro_ty_method_path(attr), vec![
                cx.expr_str(ca.span, ca.function_name.name),
                local_path(ca.span, ca.function_name),
            ])
        })).chain(custom_macros.iter().map(|cm| {
            cx.expr_call(span, proc_macro_ty_method_path(bang), vec![
                cx.expr_str(cm.span, cm.function_name.name),
                local_path(cm.span, cm.function_name),
            ])
        })).collect()
    };

    let decls_static = cx.item_static(
        span,
        Ident::from_str("_DECLS"),
        cx.ty_rptr(span,
            cx.ty(span, ast::TyKind::Slice(
                cx.ty_path(cx.path(span,
                    vec![proc_macro, bridge, client, proc_macro_ty])))),
            None, ast::Mutability::Immutable),
        ast::Mutability::Immutable,
        cx.expr_vec_slice(span, decls),
    ).map(|mut i| {
        let attr = cx.meta_word(span, Symbol::intern("rustc_proc_macro_decls"));
        i.attrs.push(cx.attribute(span, attr));
        i.vis = respan(span, ast::VisibilityKind::Public);
        i
    });

    let module = cx.item_mod(
        span,
        span,
        ast::Ident::with_empty_ctxt(Symbol::gensym("decls")),
        vec![],
        vec![krate, decls_static],
    ).map(|mut i| {
        i.vis = respan(span, ast::VisibilityKind::Public);
        i
    });

    cx.monotonic_expander().fold_item(module).pop().unwrap()
}
