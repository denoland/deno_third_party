//! The Rust AST Visitor. Extracts useful information and massages it into a form
//! usable for `clean`.

use rustc::hir::{self, Node};
use rustc::hir::def::Def;
use rustc::hir::def_id::{DefId, LOCAL_CRATE};
use rustc::middle::privacy::AccessLevel;
use rustc::util::nodemap::{FxHashSet, FxHashMap};
use syntax::ast;
use syntax::attr;
use syntax::ext::base::MacroKind;
use syntax::source_map::Spanned;
use syntax_pos::{self, Span};

use std::mem;

use core;
use clean::{self, AttributesExt, NestedAttributesExt, def_id_to_path};
use doctree::*;

// Looks to me like the first two of these are actually
// output parameters, maybe only mutated once; perhaps
// better simply to have the visit method return a tuple
// containing them?

// Also, is there some reason that this doesn't use the 'visit'
// framework from syntax?.

pub struct RustdocVisitor<'a, 'tcx: 'a, 'rcx: 'a> {
    pub module: Module,
    pub attrs: hir::HirVec<ast::Attribute>,
    pub cx: &'a core::DocContext<'a, 'tcx, 'rcx>,
    view_item_stack: FxHashSet<ast::NodeId>,
    inlining: bool,
    /// Are the current module and all of its parents public?
    inside_public_path: bool,
    exact_paths: Option<FxHashMap<DefId, Vec<String>>>,
}

impl<'a, 'tcx, 'rcx> RustdocVisitor<'a, 'tcx, 'rcx> {
    pub fn new(
        cx: &'a core::DocContext<'a, 'tcx, 'rcx>
    ) -> RustdocVisitor<'a, 'tcx, 'rcx> {
        // If the root is re-exported, terminate all recursion.
        let mut stack = FxHashSet::default();
        stack.insert(ast::CRATE_NODE_ID);
        RustdocVisitor {
            module: Module::new(None),
            attrs: hir::HirVec::new(),
            cx,
            view_item_stack: stack,
            inlining: false,
            inside_public_path: true,
            exact_paths: Some(FxHashMap::default()),
        }
    }

    fn store_path(&mut self, did: DefId) {
        // We can't use the entry API, as that keeps the mutable borrow of `self` active
        // when we try to use `cx`.
        let exact_paths = self.exact_paths.as_mut().unwrap();
        if exact_paths.get(&did).is_none() {
            let path = def_id_to_path(self.cx, did, self.cx.crate_name.clone());
            exact_paths.insert(did, path);
        }
    }

    fn stability(&self, id: ast::NodeId) -> Option<attr::Stability> {
        self.cx.tcx.hir().opt_local_def_id(id)
            .and_then(|def_id| self.cx.tcx.lookup_stability(def_id)).cloned()
    }

    fn deprecation(&self, id: ast::NodeId) -> Option<attr::Deprecation> {
        self.cx.tcx.hir().opt_local_def_id(id)
            .and_then(|def_id| self.cx.tcx.lookup_deprecation(def_id))
    }

    pub fn visit(&mut self, krate: &hir::Crate) {
        self.attrs = krate.attrs.clone();

        self.module = self.visit_mod_contents(krate.span,
                                              krate.attrs.clone(),
                                              Spanned { span: syntax_pos::DUMMY_SP,
                                                        node: hir::VisibilityKind::Public },
                                              ast::CRATE_NODE_ID,
                                              &krate.module,
                                              None);
        // Attach the crate's exported macros to the top-level module:
        let macro_exports: Vec<_> =
            krate.exported_macros.iter().map(|def| self.visit_local_macro(def, None)).collect();
        self.module.macros.extend(macro_exports);
        self.module.is_crate = true;

        self.cx.renderinfo.borrow_mut().exact_paths = self.exact_paths.take().unwrap();
    }

    pub fn visit_variant_data(&mut self, item: &hir::Item,
                              name: ast::Name, sd: &hir::VariantData,
                              generics: &hir::Generics) -> Struct {
        debug!("Visiting struct");
        let struct_type = struct_type_from_def(&*sd);
        Struct {
            id: item.id,
            struct_type,
            name,
            vis: item.vis.clone(),
            stab: self.stability(item.id),
            depr: self.deprecation(item.id),
            attrs: item.attrs.clone(),
            generics: generics.clone(),
            fields: sd.fields().iter().cloned().collect(),
            whence: item.span
        }
    }

    pub fn visit_union_data(&mut self, item: &hir::Item,
                            name: ast::Name, sd: &hir::VariantData,
                            generics: &hir::Generics) -> Union {
        debug!("Visiting union");
        let struct_type = struct_type_from_def(&*sd);
        Union {
            id: item.id,
            struct_type,
            name,
            vis: item.vis.clone(),
            stab: self.stability(item.id),
            depr: self.deprecation(item.id),
            attrs: item.attrs.clone(),
            generics: generics.clone(),
            fields: sd.fields().iter().cloned().collect(),
            whence: item.span
        }
    }

    pub fn visit_enum_def(&mut self, it: &hir::Item,
                          name: ast::Name, def: &hir::EnumDef,
                          params: &hir::Generics) -> Enum {
        debug!("Visiting enum");
        Enum {
            name,
            variants: def.variants.iter().map(|v| Variant {
                name: v.node.ident.name,
                attrs: v.node.attrs.clone(),
                stab: self.stability(v.node.data.id()),
                depr: self.deprecation(v.node.data.id()),
                def: v.node.data.clone(),
                whence: v.span,
            }).collect(),
            vis: it.vis.clone(),
            stab: self.stability(it.id),
            depr: self.deprecation(it.id),
            generics: params.clone(),
            attrs: it.attrs.clone(),
            id: it.id,
            whence: it.span,
        }
    }

    pub fn visit_fn(&mut self, om: &mut Module, item: &hir::Item,
                    name: ast::Name, fd: &hir::FnDecl,
                    header: hir::FnHeader,
                    gen: &hir::Generics,
                    body: hir::BodyId) {
        debug!("Visiting fn");
        let macro_kind = item.attrs.iter().filter_map(|a| {
            if a.check_name("proc_macro") {
                Some(MacroKind::Bang)
            } else if a.check_name("proc_macro_derive") {
                Some(MacroKind::Derive)
            } else if a.check_name("proc_macro_attribute") {
                Some(MacroKind::Attr)
            } else {
                None
            }
        }).next();
        match macro_kind {
            Some(kind) => {
                let name = if kind == MacroKind::Derive {
                    item.attrs.lists("proc_macro_derive")
                              .filter_map(|mi| mi.name())
                              .next()
                              .expect("proc-macro derives require a name")
                } else {
                    name
                };

                let mut helpers = Vec::new();
                for mi in item.attrs.lists("proc_macro_derive") {
                    if !mi.check_name("attributes") {
                        continue;
                    }

                    if let Some(list) = mi.meta_item_list() {
                        for inner_mi in list {
                            if let Some(name) = inner_mi.name() {
                                helpers.push(name);
                            }
                        }
                    }
                }

                om.proc_macros.push(ProcMacro {
                    name,
                    id: item.id,
                    kind,
                    helpers,
                    attrs: item.attrs.clone(),
                    whence: item.span,
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                });
            }
            None => {
                om.fns.push(Function {
                    id: item.id,
                    vis: item.vis.clone(),
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                    attrs: item.attrs.clone(),
                    decl: fd.clone(),
                    name,
                    whence: item.span,
                    generics: gen.clone(),
                    header,
                    body,
                });
            }
        }
    }

    pub fn visit_mod_contents(&mut self, span: Span, attrs: hir::HirVec<ast::Attribute>,
                              vis: hir::Visibility, id: ast::NodeId,
                              m: &hir::Mod,
                              name: Option<ast::Name>) -> Module {
        let mut om = Module::new(name);
        om.where_outer = span;
        om.where_inner = m.inner;
        om.attrs = attrs;
        om.vis = vis.clone();
        om.stab = self.stability(id);
        om.depr = self.deprecation(id);
        om.id = id;
        // Keep track of if there were any private modules in the path.
        let orig_inside_public_path = self.inside_public_path;
        self.inside_public_path &= vis.node.is_pub();
        for i in &m.item_ids {
            let item = self.cx.tcx.hir().expect_item(i.id);
            self.visit_item(item, None, &mut om);
        }
        self.inside_public_path = orig_inside_public_path;
        om
    }

    /// Tries to resolve the target of a `pub use` statement and inlines the
    /// target if it is defined locally and would not be documented otherwise,
    /// or when it is specifically requested with `please_inline`.
    /// (the latter is the case when the import is marked `doc(inline)`)
    ///
    /// Cross-crate inlining occurs later on during crate cleaning
    /// and follows different rules.
    ///
    /// Returns true if the target has been inlined.
    fn maybe_inline_local(&mut self,
                          id: ast::NodeId,
                          def: Def,
                          renamed: Option<ast::Ident>,
                          glob: bool,
                          om: &mut Module,
                          please_inline: bool) -> bool {

        fn inherits_doc_hidden(cx: &core::DocContext, mut node: ast::NodeId) -> bool {
            while let Some(id) = cx.tcx.hir().get_enclosing_scope(node) {
                node = id;
                if cx.tcx.hir().attrs(node).lists("doc").has_word("hidden") {
                    return true;
                }
                if node == ast::CRATE_NODE_ID {
                    break;
                }
            }
            false
        }

        debug!("maybe_inline_local def: {:?}", def);

        let tcx = self.cx.tcx;
        if def == Def::Err {
            return false;
        }
        let def_did = def.def_id();

        let use_attrs = tcx.hir().attrs(id);
        // Don't inline `doc(hidden)` imports so they can be stripped at a later stage.
        let is_no_inline = use_attrs.lists("doc").has_word("no_inline") ||
                           use_attrs.lists("doc").has_word("hidden");

        // For cross-crate impl inlining we need to know whether items are
        // reachable in documentation -- a previously nonreachable item can be
        // made reachable by cross-crate inlining which we're checking here.
        // (this is done here because we need to know this upfront).
        if !def_did.is_local() && !is_no_inline {
            let attrs = clean::inline::load_attrs(self.cx, def_did);
            let self_is_hidden = attrs.lists("doc").has_word("hidden");
            match def {
                Def::Trait(did) |
                Def::Struct(did) |
                Def::Union(did) |
                Def::Enum(did) |
                Def::ForeignTy(did) |
                Def::TyAlias(did) if !self_is_hidden => {
                    self.cx.renderinfo
                        .borrow_mut()
                        .access_levels.map
                        .insert(did, AccessLevel::Public);
                },
                Def::Mod(did) => if !self_is_hidden {
                    ::visit_lib::LibEmbargoVisitor::new(self.cx).visit_mod(did);
                },
                _ => {},
            }

            return false
        }

        let def_node_id = match tcx.hir().as_local_node_id(def_did) {
            Some(n) => n, None => return false
        };

        let is_private = !self.cx.renderinfo.borrow().access_levels.is_public(def_did);
        let is_hidden = inherits_doc_hidden(self.cx, def_node_id);

        // Only inline if requested or if the item would otherwise be stripped.
        if (!please_inline && !is_private && !is_hidden) || is_no_inline {
            return false
        }

        if !self.view_item_stack.insert(def_node_id) { return false }

        let ret = match tcx.hir().get(def_node_id) {
            Node::Item(&hir::Item { node: hir::ItemKind::Mod(ref m), .. }) if glob => {
                let prev = mem::replace(&mut self.inlining, true);
                for i in &m.item_ids {
                    let i = self.cx.tcx.hir().expect_item(i.id);
                    self.visit_item(i, None, om);
                }
                self.inlining = prev;
                true
            }
            Node::Item(it) if !glob => {
                let prev = mem::replace(&mut self.inlining, true);
                self.visit_item(it, renamed, om);
                self.inlining = prev;
                true
            }
            Node::ForeignItem(it) if !glob => {
                // Generate a fresh `extern {}` block if we want to inline a foreign item.
                om.foreigns.push(hir::ForeignMod {
                    abi: tcx.hir().get_foreign_abi(it.id),
                    items: vec![hir::ForeignItem {
                        ident: renamed.unwrap_or(it.ident),
                        .. it.clone()
                    }].into(),
                });
                true
            }
            Node::MacroDef(def) if !glob => {
                om.macros.push(self.visit_local_macro(def, renamed.map(|i| i.name)));
                true
            }
            _ => false,
        };
        self.view_item_stack.remove(&def_node_id);
        ret
    }

    pub fn visit_item(&mut self, item: &hir::Item,
                      renamed: Option<ast::Ident>, om: &mut Module) {
        debug!("Visiting item {:?}", item);
        let ident = renamed.unwrap_or(item.ident);

        if item.vis.node.is_pub() {
            let def_id = self.cx.tcx.hir().local_def_id(item.id);
            self.store_path(def_id);
        }

        match item.node {
            hir::ItemKind::ForeignMod(ref fm) => {
                // If inlining we only want to include public functions.
                om.foreigns.push(if self.inlining {
                    hir::ForeignMod {
                        abi: fm.abi,
                        items: fm.items.iter().filter(|i| i.vis.node.is_pub()).cloned().collect(),
                    }
                } else {
                    fm.clone()
                });
            }
            // If we're inlining, skip private items.
            _ if self.inlining && !item.vis.node.is_pub() => {}
            hir::ItemKind::GlobalAsm(..) => {}
            hir::ItemKind::ExternCrate(orig_name) => {
                let def_id = self.cx.tcx.hir().local_def_id(item.id);
                om.extern_crates.push(ExternCrate {
                    cnum: self.cx.tcx.extern_mod_stmt_cnum(def_id)
                                .unwrap_or(LOCAL_CRATE),
                    name: ident.name,
                    path: orig_name.map(|x|x.to_string()),
                    vis: item.vis.clone(),
                    attrs: item.attrs.clone(),
                    whence: item.span,
                })
            }
            hir::ItemKind::Use(_, hir::UseKind::ListStem) => {}
            hir::ItemKind::Use(ref path, kind) => {
                let is_glob = kind == hir::UseKind::Glob;

                // Struct and variant constructors and proc macro stubs always show up alongside
                // their definitions, we've already processed them so just discard these.
                match path.def {
                    Def::StructCtor(..) | Def::VariantCtor(..) | Def::SelfCtor(..) |
                    Def::Macro(_, MacroKind::ProcMacroStub) => return,
                    _ => {}
                }

                // If there was a private module in the current path then don't bother inlining
                // anything as it will probably be stripped anyway.
                if item.vis.node.is_pub() && self.inside_public_path {
                    let please_inline = item.attrs.iter().any(|item| {
                        match item.meta_item_list() {
                            Some(ref list) if item.check_name("doc") => {
                                list.iter().any(|i| i.check_name("inline"))
                            }
                            _ => false,
                        }
                    });
                    let ident = if is_glob { None } else { Some(ident) };
                    if self.maybe_inline_local(item.id,
                                               path.def,
                                               ident,
                                               is_glob,
                                               om,
                                               please_inline) {
                        return;
                    }
                }

                om.imports.push(Import {
                    name: ident.name,
                    id: item.id,
                    vis: item.vis.clone(),
                    attrs: item.attrs.clone(),
                    path: (**path).clone(),
                    glob: is_glob,
                    whence: item.span,
                });
            }
            hir::ItemKind::Mod(ref m) => {
                om.mods.push(self.visit_mod_contents(item.span,
                                                     item.attrs.clone(),
                                                     item.vis.clone(),
                                                     item.id,
                                                     m,
                                                     Some(ident.name)));
            },
            hir::ItemKind::Enum(ref ed, ref gen) =>
                om.enums.push(self.visit_enum_def(item, ident.name, ed, gen)),
            hir::ItemKind::Struct(ref sd, ref gen) =>
                om.structs.push(self.visit_variant_data(item, ident.name, sd, gen)),
            hir::ItemKind::Union(ref sd, ref gen) =>
                om.unions.push(self.visit_union_data(item, ident.name, sd, gen)),
            hir::ItemKind::Fn(ref fd, header, ref gen, body) =>
                self.visit_fn(om, item, ident.name, &**fd, header, gen, body),
            hir::ItemKind::Ty(ref ty, ref gen) => {
                let t = Typedef {
                    ty: ty.clone(),
                    gen: gen.clone(),
                    name: ident.name,
                    id: item.id,
                    attrs: item.attrs.clone(),
                    whence: item.span,
                    vis: item.vis.clone(),
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                };
                om.typedefs.push(t);
            },
            hir::ItemKind::Existential(ref exist_ty) => {
                let t = Existential {
                    exist_ty: exist_ty.clone(),
                    name: ident.name,
                    id: item.id,
                    attrs: item.attrs.clone(),
                    whence: item.span,
                    vis: item.vis.clone(),
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                };
                om.existentials.push(t);
            },
            hir::ItemKind::Static(ref ty, ref mut_, ref exp) => {
                let s = Static {
                    type_: ty.clone(),
                    mutability: mut_.clone(),
                    expr: exp.clone(),
                    id: item.id,
                    name: ident.name,
                    attrs: item.attrs.clone(),
                    whence: item.span,
                    vis: item.vis.clone(),
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                };
                om.statics.push(s);
            },
            hir::ItemKind::Const(ref ty, ref exp) => {
                let s = Constant {
                    type_: ty.clone(),
                    expr: exp.clone(),
                    id: item.id,
                    name: ident.name,
                    attrs: item.attrs.clone(),
                    whence: item.span,
                    vis: item.vis.clone(),
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                };
                om.constants.push(s);
            },
            hir::ItemKind::Trait(is_auto, unsafety, ref gen, ref b, ref item_ids) => {
                let items = item_ids.iter()
                                    .map(|ti| self.cx.tcx.hir().trait_item(ti.id).clone())
                                    .collect();
                let t = Trait {
                    is_auto,
                    unsafety,
                    name: ident.name,
                    items,
                    generics: gen.clone(),
                    bounds: b.iter().cloned().collect(),
                    id: item.id,
                    attrs: item.attrs.clone(),
                    whence: item.span,
                    vis: item.vis.clone(),
                    stab: self.stability(item.id),
                    depr: self.deprecation(item.id),
                };
                om.traits.push(t);
            },
            hir::ItemKind::TraitAlias(..) => {
                unimplemented!("trait objects are not yet implemented")
            },

            hir::ItemKind::Impl(unsafety,
                          polarity,
                          defaultness,
                          ref gen,
                          ref tr,
                          ref ty,
                          ref item_ids) => {
                // Don't duplicate impls when inlining or if it's implementing a trait, we'll pick
                // them up regardless of where they're located.
                if !self.inlining && tr.is_none() {
                    let items = item_ids.iter()
                                        .map(|ii| self.cx.tcx.hir().impl_item(ii.id).clone())
                                        .collect();
                    let i = Impl {
                        unsafety,
                        polarity,
                        defaultness,
                        generics: gen.clone(),
                        trait_: tr.clone(),
                        for_: ty.clone(),
                        items,
                        attrs: item.attrs.clone(),
                        id: item.id,
                        whence: item.span,
                        vis: item.vis.clone(),
                        stab: self.stability(item.id),
                        depr: self.deprecation(item.id),
                    };
                    om.impls.push(i);
                }
            },
        }
    }

    // Convert each `exported_macro` into a doc item.
    fn visit_local_macro(
        &self,
        def: &hir::MacroDef,
        renamed: Option<ast::Name>
    ) -> Macro {
        debug!("visit_local_macro: {}", def.name);
        let tts = def.body.trees().collect::<Vec<_>>();
        // Extract the spans of all matchers. They represent the "interface" of the macro.
        let matchers = tts.chunks(4).map(|arm| arm[0].span()).collect();

        Macro {
            def_id: self.cx.tcx.hir().local_def_id(def.id),
            attrs: def.attrs.clone(),
            name: renamed.unwrap_or(def.name),
            whence: def.span,
            matchers,
            stab: self.stability(def.id),
            depr: self.deprecation(def.id),
            imported_from: None,
        }
    }
}
