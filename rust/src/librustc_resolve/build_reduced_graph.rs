//! Reduced graph building
//!
//! Here we build the "reduced graph": the graph of the module tree without
//! any imports resolved.

use macros::{InvocationData, ParentScope, LegacyScope};
use resolve_imports::ImportDirective;
use resolve_imports::ImportDirectiveSubclass::{self, GlobImport, SingleImport};
use {Module, ModuleData, ModuleKind, NameBinding, NameBindingKind, Segment, ToNameBinding};
use {ModuleOrUniformRoot, PerNS, Resolver, ResolverArenas, ExternPreludeEntry};
use Namespace::{self, TypeNS, ValueNS, MacroNS};
use {resolve_error, resolve_struct_error, ResolutionError};

use rustc::hir::def::*;
use rustc::hir::def_id::{CrateNum, CRATE_DEF_INDEX, LOCAL_CRATE, DefId};
use rustc::ty;
use rustc::middle::cstore::CrateStore;
use rustc_metadata::cstore::LoadedMacro;

use std::cell::Cell;
use std::ptr;
use rustc_data_structures::sync::Lrc;

use syntax::ast::{Name, Ident};
use syntax::attr;

use syntax::ast::{self, Block, ForeignItem, ForeignItemKind, Item, ItemKind, NodeId};
use syntax::ast::{MetaItemKind, Mutability, StmtKind, TraitItem, TraitItemKind, Variant};
use syntax::ext::base::{MacroKind, SyntaxExtension};
use syntax::ext::base::Determinacy::Undetermined;
use syntax::ext::hygiene::Mark;
use syntax::ext::tt::macro_rules;
use syntax::feature_gate::{is_builtin_attr, emit_feature_err, GateIssue};
use syntax::parse::token::{self, Token};
use syntax::std_inject::injected_crate_name;
use syntax::symbol::keywords;
use syntax::visit::{self, Visitor};

use syntax_pos::{Span, DUMMY_SP};

impl<'a> ToNameBinding<'a> for (Module<'a>, ty::Visibility, Span, Mark) {
    fn to_name_binding(self, arenas: &'a ResolverArenas<'a>) -> &'a NameBinding<'a> {
        arenas.alloc_name_binding(NameBinding {
            kind: NameBindingKind::Module(self.0),
            ambiguity: None,
            vis: self.1,
            span: self.2,
            expansion: self.3,
        })
    }
}

impl<'a> ToNameBinding<'a> for (Def, ty::Visibility, Span, Mark) {
    fn to_name_binding(self, arenas: &'a ResolverArenas<'a>) -> &'a NameBinding<'a> {
        arenas.alloc_name_binding(NameBinding {
            kind: NameBindingKind::Def(self.0, false),
            ambiguity: None,
            vis: self.1,
            span: self.2,
            expansion: self.3,
        })
    }
}

pub(crate) struct IsMacroExport;

impl<'a> ToNameBinding<'a> for (Def, ty::Visibility, Span, Mark, IsMacroExport) {
    fn to_name_binding(self, arenas: &'a ResolverArenas<'a>) -> &'a NameBinding<'a> {
        arenas.alloc_name_binding(NameBinding {
            kind: NameBindingKind::Def(self.0, true),
            ambiguity: None,
            vis: self.1,
            span: self.2,
            expansion: self.3,
        })
    }
}

impl<'a> Resolver<'a> {
    /// Defines `name` in namespace `ns` of module `parent` to be `def` if it is not yet defined;
    /// otherwise, reports an error.
    pub fn define<T>(&mut self, parent: Module<'a>, ident: Ident, ns: Namespace, def: T)
        where T: ToNameBinding<'a>,
    {
        let binding = def.to_name_binding(self.arenas);
        if let Err(old_binding) = self.try_define(parent, ident, ns, binding) {
            self.report_conflict(parent, ident, ns, old_binding, &binding);
        }
    }

    fn block_needs_anonymous_module(&mut self, block: &Block) -> bool {
        // If any statements are items, we need to create an anonymous module
        block.stmts.iter().any(|statement| match statement.node {
            StmtKind::Item(_) | StmtKind::Mac(_) => true,
            _ => false,
        })
    }

    fn insert_field_names(&mut self, def_id: DefId, field_names: Vec<Name>) {
        if !field_names.is_empty() {
            self.field_names.insert(def_id, field_names);
        }
    }

    fn build_reduced_graph_for_use_tree(
        &mut self,
        // This particular use tree
        use_tree: &ast::UseTree,
        id: NodeId,
        parent_prefix: &[Segment],
        nested: bool,
        // The whole `use` item
        parent_scope: ParentScope<'a>,
        item: &Item,
        vis: ty::Visibility,
        root_span: Span,
    ) {
        debug!("build_reduced_graph_for_use_tree(parent_prefix={:?}, use_tree={:?}, nested={})",
               parent_prefix, use_tree, nested);

        let mut prefix_iter = parent_prefix.iter().cloned()
            .chain(use_tree.prefix.segments.iter().map(|seg| seg.into())).peekable();

        // On 2015 edition imports are resolved as crate-relative by default,
        // so prefixes are prepended with crate root segment if necessary.
        // The root is prepended lazily, when the first non-empty prefix or terminating glob
        // appears, so imports in braced groups can have roots prepended independently.
        // 2015 identifiers used on global 2018 edition enter special "virtual 2015 mode", don't
        // get crate root prepended, but get special treatment during in-scope resolution instead.
        let is_glob = if let ast::UseTreeKind::Glob = use_tree.kind { true } else { false };
        let crate_root = match prefix_iter.peek() {
            Some(seg) if !seg.ident.is_path_segment_keyword() &&
                         seg.ident.span.rust_2015() && self.session.rust_2015() => {
                Some(seg.ident.span.ctxt())
            }
            None if is_glob && use_tree.span.rust_2015() => {
                Some(use_tree.span.ctxt())
            }
            _ => None,
        }.map(|ctxt| Segment::from_ident(Ident::new(
            keywords::PathRoot.name(), use_tree.prefix.span.shrink_to_lo().with_ctxt(ctxt)
        )));

        let prefix = crate_root.into_iter().chain(prefix_iter).collect::<Vec<_>>();
        debug!("build_reduced_graph_for_use_tree: prefix={:?}", prefix);

        let empty_for_self = |prefix: &[Segment]| {
            prefix.is_empty() ||
            prefix.len() == 1 && prefix[0].ident.name == keywords::PathRoot.name()
        };
        match use_tree.kind {
            ast::UseTreeKind::Simple(rename, ..) => {
                let mut ident = use_tree.ident().gensym_if_underscore();
                let mut module_path = prefix;
                let mut source = module_path.pop().unwrap();
                let mut type_ns_only = false;

                if nested {
                    // Correctly handle `self`
                    if source.ident.name == keywords::SelfLower.name() {
                        type_ns_only = true;

                        if empty_for_self(&module_path) {
                            resolve_error(
                                self,
                                use_tree.span,
                                ResolutionError::
                                SelfImportOnlyInImportListWithNonEmptyPrefix
                            );
                            return;
                        }

                        // Replace `use foo::self;` with `use foo;`
                        source = module_path.pop().unwrap();
                        if rename.is_none() {
                            ident = source.ident;
                        }
                    }
                } else {
                    // Disallow `self`
                    if source.ident.name == keywords::SelfLower.name() {
                        resolve_error(self,
                                      use_tree.span,
                                      ResolutionError::SelfImportsOnlyAllowedWithin);
                    }

                    // Disallow `use $crate;`
                    if source.ident.name == keywords::DollarCrate.name() && module_path.is_empty() {
                        let crate_root = self.resolve_crate_root(source.ident);
                        let crate_name = match crate_root.kind {
                            ModuleKind::Def(_, name) => name,
                            ModuleKind::Block(..) => unreachable!(),
                        };
                        // HACK(eddyb) unclear how good this is, but keeping `$crate`
                        // in `source` breaks `src/test/compile-fail/import-crate-var.rs`,
                        // while the current crate doesn't have a valid `crate_name`.
                        if crate_name != keywords::Invalid.name() {
                            // `crate_name` should not be interpreted as relative.
                            module_path.push(Segment {
                                ident: Ident {
                                    name: keywords::PathRoot.name(),
                                    span: source.ident.span,
                                },
                                id: Some(self.session.next_node_id()),
                            });
                            source.ident.name = crate_name;
                        }
                        if rename.is_none() {
                            ident.name = crate_name;
                        }

                        self.session.struct_span_warn(item.span, "`$crate` may not be imported")
                            .note("`use $crate;` was erroneously allowed and \
                                   will become a hard error in a future release")
                            .emit();
                    }
                }

                if ident.name == keywords::Crate.name() {
                    self.session.span_err(ident.span,
                        "crate root imports need to be explicitly named: \
                         `use crate as name;`");
                }

                let subclass = SingleImport {
                    source: source.ident,
                    target: ident,
                    source_bindings: PerNS {
                        type_ns: Cell::new(Err(Undetermined)),
                        value_ns: Cell::new(Err(Undetermined)),
                        macro_ns: Cell::new(Err(Undetermined)),
                    },
                    target_bindings: PerNS {
                        type_ns: Cell::new(None),
                        value_ns: Cell::new(None),
                        macro_ns: Cell::new(None),
                    },
                    type_ns_only,
                };
                self.add_import_directive(
                    module_path,
                    subclass,
                    use_tree.span,
                    id,
                    root_span,
                    item.id,
                    vis,
                    parent_scope,
                );
            }
            ast::UseTreeKind::Glob => {
                let subclass = GlobImport {
                    is_prelude: attr::contains_name(&item.attrs, "prelude_import"),
                    max_vis: Cell::new(ty::Visibility::Invisible),
                };
                self.add_import_directive(
                    prefix,
                    subclass,
                    use_tree.span,
                    id,
                    root_span,
                    item.id,
                    vis,
                    parent_scope,
                );
            }
            ast::UseTreeKind::Nested(ref items) => {
                // Ensure there is at most one `self` in the list
                let self_spans = items.iter().filter_map(|&(ref use_tree, _)| {
                    if let ast::UseTreeKind::Simple(..) = use_tree.kind {
                        if use_tree.ident().name == keywords::SelfLower.name() {
                            return Some(use_tree.span);
                        }
                    }

                    None
                }).collect::<Vec<_>>();
                if self_spans.len() > 1 {
                    let mut e = resolve_struct_error(self,
                        self_spans[0],
                        ResolutionError::SelfImportCanOnlyAppearOnceInTheList);

                    for other_span in self_spans.iter().skip(1) {
                        e.span_label(*other_span, "another `self` import appears here");
                    }

                    e.emit();
                }

                for &(ref tree, id) in items {
                    self.build_reduced_graph_for_use_tree(
                        // This particular use tree
                        tree, id, &prefix, true,
                        // The whole `use` item
                        parent_scope.clone(), item, vis, root_span,
                    );
                }

                // Empty groups `a::b::{}` are turned into synthetic `self` imports
                // `a::b::c::{self as _}`, so that their prefixes are correctly
                // resolved and checked for privacy/stability/etc.
                if items.is_empty() && !empty_for_self(&prefix) {
                    let new_span = prefix[prefix.len() - 1].ident.span;
                    let tree = ast::UseTree {
                        prefix: ast::Path::from_ident(
                            Ident::new(keywords::SelfLower.name(), new_span)
                        ),
                        kind: ast::UseTreeKind::Simple(
                            Some(Ident::new(keywords::Underscore.name().gensymed(), new_span)),
                            ast::DUMMY_NODE_ID,
                            ast::DUMMY_NODE_ID,
                        ),
                        span: use_tree.span,
                    };
                    self.build_reduced_graph_for_use_tree(
                        // This particular use tree
                        &tree, id, &prefix, true,
                        // The whole `use` item
                        parent_scope, item, ty::Visibility::Invisible, root_span,
                    );
                }
            }
        }
    }

    /// Constructs the reduced graph for one item.
    fn build_reduced_graph_for_item(&mut self, item: &Item, parent_scope: ParentScope<'a>) {
        let parent = parent_scope.module;
        let expansion = parent_scope.expansion;
        let ident = item.ident.gensym_if_underscore();
        let sp = item.span;
        let vis = self.resolve_visibility(&item.vis);

        match item.node {
            ItemKind::Use(ref use_tree) => {
                self.build_reduced_graph_for_use_tree(
                    // This particular use tree
                    use_tree, item.id, &[], false,
                    // The whole `use` item
                    parent_scope, item, vis, use_tree.span,
                );
            }

            ItemKind::ExternCrate(orig_name) => {
                let module = if orig_name.is_none() && ident.name == keywords::SelfLower.name() {
                    self.session
                        .struct_span_err(item.span, "`extern crate self;` requires renaming")
                        .span_suggestion(item.span, "try", "extern crate self as name;".into())
                        .emit();
                    return;
                } else if orig_name == Some(keywords::SelfLower.name()) {
                    if !self.session.features_untracked().extern_crate_self {
                        emit_feature_err(&self.session.parse_sess, "extern_crate_self", item.span,
                                         GateIssue::Language, "`extern crate self` is unstable");
                    }
                    self.graph_root
                } else {
                    let crate_id = self.crate_loader.process_extern_crate(item, &self.definitions);
                    self.get_module(DefId { krate: crate_id, index: CRATE_DEF_INDEX })
                };

                self.populate_module_if_necessary(module);
                if injected_crate_name().map_or(false, |name| ident.name == name) {
                    self.injected_crate = Some(module);
                }

                let used = self.process_legacy_macro_imports(item, module, &parent_scope);
                let binding =
                    (module, ty::Visibility::Public, sp, expansion).to_name_binding(self.arenas);
                let directive = self.arenas.alloc_import_directive(ImportDirective {
                    root_id: item.id,
                    id: item.id,
                    parent_scope,
                    imported_module: Cell::new(Some(ModuleOrUniformRoot::Module(module))),
                    subclass: ImportDirectiveSubclass::ExternCrate {
                        source: orig_name,
                        target: ident,
                    },
                    root_span: item.span,
                    span: item.span,
                    module_path: Vec::new(),
                    vis: Cell::new(vis),
                    used: Cell::new(used),
                });
                self.potentially_unused_imports.push(directive);
                let imported_binding = self.import(binding, directive);
                if ptr::eq(self.current_module, self.graph_root) {
                    if let Some(entry) = self.extern_prelude.get(&ident.modern()) {
                        if expansion != Mark::root() && orig_name.is_some() &&
                           entry.extern_crate_item.is_none() {
                            self.session.span_err(item.span, "macro-expanded `extern crate` items \
                                                              cannot shadow names passed with \
                                                              `--extern`");
                        }
                    }
                    let entry = self.extern_prelude.entry(ident.modern())
                                                   .or_insert(ExternPreludeEntry {
                        extern_crate_item: None,
                        introduced_by_item: true,
                    });
                    entry.extern_crate_item = Some(imported_binding);
                    if orig_name.is_some() {
                        entry.introduced_by_item = true;
                    }
                }
                self.define(parent, ident, TypeNS, imported_binding);
            }

            ItemKind::GlobalAsm(..) => {}

            ItemKind::Mod(..) if ident == keywords::Invalid.ident() => {} // Crate root

            ItemKind::Mod(..) => {
                let def_id = self.definitions.local_def_id(item.id);
                let module_kind = ModuleKind::Def(Def::Mod(def_id), ident.name);
                let module = self.arenas.alloc_module(ModuleData {
                    no_implicit_prelude: parent.no_implicit_prelude || {
                        attr::contains_name(&item.attrs, "no_implicit_prelude")
                    },
                    ..ModuleData::new(Some(parent), module_kind, def_id, expansion, item.span)
                });
                self.define(parent, ident, TypeNS, (module, vis, sp, expansion));
                self.module_map.insert(def_id, module);

                // Descend into the module.
                self.current_module = module;
            }

            // Handled in `rustc_metadata::{native_libs,link_args}`
            ItemKind::ForeignMod(..) => {}

            // These items live in the value namespace.
            ItemKind::Static(_, m, _) => {
                let mutbl = m == Mutability::Mutable;
                let def = Def::Static(self.definitions.local_def_id(item.id), mutbl);
                self.define(parent, ident, ValueNS, (def, vis, sp, expansion));
            }
            ItemKind::Const(..) => {
                let def = Def::Const(self.definitions.local_def_id(item.id));
                self.define(parent, ident, ValueNS, (def, vis, sp, expansion));
            }
            ItemKind::Fn(..) => {
                let def = Def::Fn(self.definitions.local_def_id(item.id));
                self.define(parent, ident, ValueNS, (def, vis, sp, expansion));

                // Functions introducing procedural macros reserve a slot
                // in the macro namespace as well (see #52225).
                if attr::contains_name(&item.attrs, "proc_macro") ||
                   attr::contains_name(&item.attrs, "proc_macro_attribute") {
                    let def = Def::Macro(def.def_id(), MacroKind::ProcMacroStub);
                    self.define(parent, ident, MacroNS, (def, vis, sp, expansion));
                }
                if let Some(attr) = attr::find_by_name(&item.attrs, "proc_macro_derive") {
                    if let Some(trait_attr) =
                            attr.meta_item_list().and_then(|list| list.get(0).cloned()) {
                        if let Some(ident) = trait_attr.name().map(Ident::with_empty_ctxt) {
                            let sp = trait_attr.span;
                            let def = Def::Macro(def.def_id(), MacroKind::ProcMacroStub);
                            self.define(parent, ident, MacroNS, (def, vis, sp, expansion));
                        }
                    }
                }
            }

            // These items live in the type namespace.
            ItemKind::Ty(..) => {
                let def = Def::TyAlias(self.definitions.local_def_id(item.id));
                self.define(parent, ident, TypeNS, (def, vis, sp, expansion));
            }

            ItemKind::Existential(_, _) => {
                let def = Def::Existential(self.definitions.local_def_id(item.id));
                self.define(parent, ident, TypeNS, (def, vis, sp, expansion));
            }

            ItemKind::Enum(ref enum_definition, _) => {
                let def = Def::Enum(self.definitions.local_def_id(item.id));
                let module_kind = ModuleKind::Def(def, ident.name);
                let module = self.new_module(parent,
                                             module_kind,
                                             parent.normal_ancestor_id,
                                             expansion,
                                             item.span);
                self.define(parent, ident, TypeNS, (module, vis, sp, expansion));

                for variant in &(*enum_definition).variants {
                    self.build_reduced_graph_for_variant(variant, module, vis, expansion);
                }
            }

            ItemKind::TraitAlias(..) => {
                let def = Def::TraitAlias(self.definitions.local_def_id(item.id));
                self.define(parent, ident, TypeNS, (def, vis, sp, expansion));
            }

            // These items live in both the type and value namespaces.
            ItemKind::Struct(ref struct_def, _) => {
                // Define a name in the type namespace.
                let def_id = self.definitions.local_def_id(item.id);
                let def = Def::Struct(def_id);
                self.define(parent, ident, TypeNS, (def, vis, sp, expansion));

                let mut ctor_vis = vis;

                let has_non_exhaustive = attr::contains_name(&item.attrs, "non_exhaustive");

                // If the structure is marked as non_exhaustive then lower the visibility
                // to within the crate.
                if has_non_exhaustive && vis == ty::Visibility::Public {
                    ctor_vis = ty::Visibility::Restricted(DefId::local(CRATE_DEF_INDEX));
                }

                // Record field names for error reporting.
                let field_names = struct_def.fields().iter().filter_map(|field| {
                    let field_vis = self.resolve_visibility(&field.vis);
                    if ctor_vis.is_at_least(field_vis, &*self) {
                        ctor_vis = field_vis;
                    }
                    field.ident.map(|ident| ident.name)
                }).collect();
                let item_def_id = self.definitions.local_def_id(item.id);
                self.insert_field_names(item_def_id, field_names);

                // If this is a tuple or unit struct, define a name
                // in the value namespace as well.
                if !struct_def.is_struct() {
                    let ctor_def = Def::StructCtor(self.definitions.local_def_id(struct_def.id()),
                                                   CtorKind::from_ast(struct_def));
                    self.define(parent, ident, ValueNS, (ctor_def, ctor_vis, sp, expansion));
                    self.struct_constructors.insert(def.def_id(), (ctor_def, ctor_vis));
                }
            }

            ItemKind::Union(ref vdata, _) => {
                let def = Def::Union(self.definitions.local_def_id(item.id));
                self.define(parent, ident, TypeNS, (def, vis, sp, expansion));

                // Record field names for error reporting.
                let field_names = vdata.fields().iter().filter_map(|field| {
                    self.resolve_visibility(&field.vis);
                    field.ident.map(|ident| ident.name)
                }).collect();
                let item_def_id = self.definitions.local_def_id(item.id);
                self.insert_field_names(item_def_id, field_names);
            }

            ItemKind::Impl(..) => {}

            ItemKind::Trait(..) => {
                let def_id = self.definitions.local_def_id(item.id);

                // Add all the items within to a new module.
                let module_kind = ModuleKind::Def(Def::Trait(def_id), ident.name);
                let module = self.new_module(parent,
                                             module_kind,
                                             parent.normal_ancestor_id,
                                             expansion,
                                             item.span);
                self.define(parent, ident, TypeNS, (module, vis, sp, expansion));
                self.current_module = module;
            }

            ItemKind::MacroDef(..) | ItemKind::Mac(_) => unreachable!(),
        }
    }

    // Constructs the reduced graph for one variant. Variants exist in the
    // type and value namespaces.
    fn build_reduced_graph_for_variant(&mut self,
                                       variant: &Variant,
                                       parent: Module<'a>,
                                       vis: ty::Visibility,
                                       expansion: Mark) {
        let ident = variant.node.ident;
        let def_id = self.definitions.local_def_id(variant.node.data.id());

        // Define a name in the type namespace.
        let def = Def::Variant(def_id);
        self.define(parent, ident, TypeNS, (def, vis, variant.span, expansion));

        // Define a constructor name in the value namespace.
        // Braced variants, unlike structs, generate unusable names in
        // value namespace, they are reserved for possible future use.
        let ctor_kind = CtorKind::from_ast(&variant.node.data);
        let ctor_def = Def::VariantCtor(def_id, ctor_kind);

        self.define(parent, ident, ValueNS, (ctor_def, vis, variant.span, expansion));
    }

    /// Constructs the reduced graph for one foreign item.
    fn build_reduced_graph_for_foreign_item(&mut self, item: &ForeignItem, expansion: Mark) {
        let (def, ns) = match item.node {
            ForeignItemKind::Fn(..) => {
                (Def::Fn(self.definitions.local_def_id(item.id)), ValueNS)
            }
            ForeignItemKind::Static(_, m) => {
                (Def::Static(self.definitions.local_def_id(item.id), m), ValueNS)
            }
            ForeignItemKind::Ty => {
                (Def::ForeignTy(self.definitions.local_def_id(item.id)), TypeNS)
            }
            ForeignItemKind::Macro(_) => unreachable!(),
        };
        let parent = self.current_module;
        let vis = self.resolve_visibility(&item.vis);
        self.define(parent, item.ident, ns, (def, vis, item.span, expansion));
    }

    fn build_reduced_graph_for_block(&mut self, block: &Block, expansion: Mark) {
        let parent = self.current_module;
        if self.block_needs_anonymous_module(block) {
            let module = self.new_module(parent,
                                         ModuleKind::Block(block.id),
                                         parent.normal_ancestor_id,
                                         expansion,
                                         block.span);
            self.block_map.insert(block.id, module);
            self.current_module = module; // Descend into the block.
        }
    }

    /// Builds the reduced graph for a single item in an external crate.
    fn build_reduced_graph_for_external_crate_def(&mut self, parent: Module<'a>, child: Export) {
        let Export { ident, def, vis, span } = child;
        // FIXME: We shouldn't create the gensym here, it should come from metadata,
        // but metadata cannot encode gensyms currently, so we create it here.
        // This is only a guess, two equivalent idents may incorrectly get different gensyms here.
        let ident = ident.gensym_if_underscore();
        let def_id = def.def_id();
        let expansion = Mark::root(); // FIXME(jseyfried) intercrate hygiene
        match def {
            Def::Mod(..) | Def::Enum(..) => {
                let module = self.new_module(parent,
                                             ModuleKind::Def(def, ident.name),
                                             def_id,
                                             expansion,
                                             span);
                self.define(parent, ident, TypeNS, (module, vis, DUMMY_SP, expansion));
            }
            Def::Variant(..) | Def::TyAlias(..) | Def::ForeignTy(..) => {
                self.define(parent, ident, TypeNS, (def, vis, DUMMY_SP, expansion));
            }
            Def::Fn(..) | Def::Static(..) | Def::Const(..) | Def::VariantCtor(..) => {
                self.define(parent, ident, ValueNS, (def, vis, DUMMY_SP, expansion));
            }
            Def::StructCtor(..) => {
                self.define(parent, ident, ValueNS, (def, vis, DUMMY_SP, expansion));

                if let Some(struct_def_id) =
                        self.cstore.def_key(def_id).parent
                            .map(|index| DefId { krate: def_id.krate, index: index }) {
                    self.struct_constructors.insert(struct_def_id, (def, vis));
                }
            }
            Def::Trait(..) => {
                let module_kind = ModuleKind::Def(def, ident.name);
                let module = self.new_module(parent,
                                             module_kind,
                                             parent.normal_ancestor_id,
                                             expansion,
                                             span);
                self.define(parent, ident, TypeNS, (module, vis, DUMMY_SP, expansion));

                for child in self.cstore.item_children_untracked(def_id, self.session) {
                    let ns = if let Def::AssociatedTy(..) = child.def { TypeNS } else { ValueNS };
                    self.define(module, child.ident, ns,
                                (child.def, ty::Visibility::Public, DUMMY_SP, expansion));

                    if self.cstore.associated_item_cloned_untracked(child.def.def_id())
                           .method_has_self_argument {
                        self.has_self.insert(child.def.def_id());
                    }
                }
                module.populated.set(true);
            }
            Def::Struct(..) | Def::Union(..) => {
                self.define(parent, ident, TypeNS, (def, vis, DUMMY_SP, expansion));

                // Record field names for error reporting.
                let field_names = self.cstore.struct_field_names_untracked(def_id);
                self.insert_field_names(def_id, field_names);
            }
            Def::Macro(..) => {
                self.define(parent, ident, MacroNS, (def, vis, DUMMY_SP, expansion));
            }
            _ => bug!("unexpected definition: {:?}", def)
        }
    }

    pub fn get_module(&mut self, def_id: DefId) -> Module<'a> {
        if def_id.krate == LOCAL_CRATE {
            return self.module_map[&def_id]
        }

        let macros_only = self.cstore.dep_kind_untracked(def_id.krate).macros_only();
        if let Some(&module) = self.extern_module_map.get(&(def_id, macros_only)) {
            return module;
        }

        let (name, parent) = if def_id.index == CRATE_DEF_INDEX {
            (self.cstore.crate_name_untracked(def_id.krate).as_interned_str(), None)
        } else {
            let def_key = self.cstore.def_key(def_id);
            (def_key.disambiguated_data.data.get_opt_name().unwrap(),
             Some(self.get_module(DefId { index: def_key.parent.unwrap(), ..def_id })))
        };

        let kind = ModuleKind::Def(Def::Mod(def_id), name.as_symbol());
        let module =
            self.arenas.alloc_module(ModuleData::new(parent, kind, def_id, Mark::root(), DUMMY_SP));
        self.extern_module_map.insert((def_id, macros_only), module);
        module
    }

    pub fn macro_def_scope(&mut self, expansion: Mark) -> Module<'a> {
        let def_id = self.macro_defs[&expansion];
        if let Some(id) = self.definitions.as_local_node_id(def_id) {
            self.local_macro_def_scopes[&id]
        } else if def_id.krate == CrateNum::BuiltinMacros {
            self.injected_crate.unwrap_or(self.graph_root)
        } else {
            let module_def_id = ty::DefIdTree::parent(&*self, def_id).unwrap();
            self.get_module(module_def_id)
        }
    }

    pub fn get_macro(&mut self, def: Def) -> Lrc<SyntaxExtension> {
        let def_id = match def {
            Def::Macro(def_id, ..) => def_id,
            Def::NonMacroAttr(attr_kind) => return Lrc::new(SyntaxExtension::NonMacroAttr {
                mark_used: attr_kind == NonMacroAttrKind::Tool,
            }),
            _ => panic!("expected `Def::Macro` or `Def::NonMacroAttr`"),
        };
        if let Some(ext) = self.macro_map.get(&def_id) {
            return ext.clone();
        }

        let macro_def = match self.cstore.load_macro_untracked(def_id, &self.session) {
            LoadedMacro::MacroDef(macro_def) => macro_def,
            LoadedMacro::ProcMacro(ext) => return ext,
        };

        let ext = Lrc::new(macro_rules::compile(&self.session.parse_sess,
                                               &self.session.features_untracked(),
                                               &macro_def,
                                               self.cstore.crate_edition_untracked(def_id.krate)));
        self.macro_map.insert(def_id, ext.clone());
        ext
    }

    /// Ensures that the reduced graph rooted at the given external module
    /// is built, building it if it is not.
    pub fn populate_module_if_necessary(&mut self, module: Module<'a>) {
        if module.populated.get() { return }
        let def_id = module.def_id().unwrap();
        for child in self.cstore.item_children_untracked(def_id, self.session) {
            self.build_reduced_graph_for_external_crate_def(module, child);
        }
        module.populated.set(true)
    }

    fn legacy_import_macro(&mut self,
                           name: Name,
                           binding: &'a NameBinding<'a>,
                           span: Span,
                           allow_shadowing: bool) {
        if self.macro_use_prelude.insert(name, binding).is_some() && !allow_shadowing {
            let msg = format!("`{}` is already in scope", name);
            let note =
                "macro-expanded `#[macro_use]`s may not shadow existing macros (see RFC 1560)";
            self.session.struct_span_err(span, &msg).note(note).emit();
        }
    }

    // This returns true if we should consider the underlying `extern crate` to be used.
    fn process_legacy_macro_imports(&mut self, item: &Item, module: Module<'a>,
                                    parent_scope: &ParentScope<'a>) -> bool {
        let mut import_all = None;
        let mut single_imports = Vec::new();
        for attr in &item.attrs {
            if attr.check_name("macro_use") {
                if self.current_module.parent.is_some() {
                    span_err!(self.session, item.span, E0468,
                        "an `extern crate` loading macros must be at the crate root");
                }
                if let ItemKind::ExternCrate(Some(orig_name)) = item.node {
                    if orig_name == keywords::SelfLower.name() {
                        self.session.span_err(attr.span,
                            "`macro_use` is not supported on `extern crate self`");
                    }
                }
                let ill_formed = |span| span_err!(self.session, span, E0466, "bad macro import");
                match attr.meta() {
                    Some(meta) => match meta.node {
                        MetaItemKind::Word => {
                            import_all = Some(meta.span);
                            break;
                        }
                        MetaItemKind::List(nested_metas) => for nested_meta in nested_metas {
                            match nested_meta.word() {
                                Some(word) => single_imports.push((word.name(), word.span)),
                                None => ill_formed(nested_meta.span),
                            }
                        }
                        MetaItemKind::NameValue(..) => ill_formed(meta.span),
                    }
                    None => ill_formed(attr.span()),
                }
            }
        }

        let arenas = self.arenas;
        let macro_use_directive = |span| arenas.alloc_import_directive(ImportDirective {
            root_id: item.id,
            id: item.id,
            parent_scope: parent_scope.clone(),
            imported_module: Cell::new(Some(ModuleOrUniformRoot::Module(module))),
            subclass: ImportDirectiveSubclass::MacroUse,
            root_span: span,
            span,
            module_path: Vec::new(),
            vis: Cell::new(ty::Visibility::Restricted(DefId::local(CRATE_DEF_INDEX))),
            used: Cell::new(false),
        });

        let allow_shadowing = parent_scope.expansion == Mark::root();
        if let Some(span) = import_all {
            let directive = macro_use_directive(span);
            self.potentially_unused_imports.push(directive);
            module.for_each_child(|ident, ns, binding| if ns == MacroNS {
                let imported_binding = self.import(binding, directive);
                self.legacy_import_macro(ident.name, imported_binding, span, allow_shadowing);
            });
        } else {
            for (name, span) in single_imports.iter().cloned() {
                let ident = Ident::with_empty_ctxt(name);
                let result = self.resolve_ident_in_module(
                    ModuleOrUniformRoot::Module(module),
                    ident,
                    MacroNS,
                    None,
                    false,
                    span,
                );
                if let Ok(binding) = result {
                    let directive = macro_use_directive(span);
                    self.potentially_unused_imports.push(directive);
                    let imported_binding = self.import(binding, directive);
                    self.legacy_import_macro(name, imported_binding, span, allow_shadowing);
                } else {
                    span_err!(self.session, span, E0469, "imported macro not found");
                }
            }
        }
        import_all.is_some() || !single_imports.is_empty()
    }

    // does this attribute list contain "macro_use"?
    fn contains_macro_use(&mut self, attrs: &[ast::Attribute]) -> bool {
        for attr in attrs {
            if attr.check_name("macro_escape") {
                let msg = "macro_escape is a deprecated synonym for macro_use";
                let mut err = self.session.struct_span_warn(attr.span, msg);
                if let ast::AttrStyle::Inner = attr.style {
                    err.help("consider an outer attribute, #[macro_use] mod ...").emit();
                } else {
                    err.emit();
                }
            } else if !attr.check_name("macro_use") {
                continue;
            }

            if !attr.is_word() {
                self.session.span_err(attr.span, "arguments to macro_use are not allowed here");
            }
            return true;
        }

        false
    }
}

pub struct BuildReducedGraphVisitor<'a, 'b: 'a> {
    pub resolver: &'a mut Resolver<'b>,
    pub current_legacy_scope: LegacyScope<'b>,
    pub expansion: Mark,
}

impl<'a, 'b> BuildReducedGraphVisitor<'a, 'b> {
    fn visit_invoc(&mut self, id: ast::NodeId) -> &'b InvocationData<'b> {
        let mark = id.placeholder_to_mark();
        self.resolver.current_module.unresolved_invocations.borrow_mut().insert(mark);
        let invocation = self.resolver.invocations[&mark];
        invocation.module.set(self.resolver.current_module);
        invocation.parent_legacy_scope.set(self.current_legacy_scope);
        invocation
    }
}

macro_rules! method {
    ($visit:ident: $ty:ty, $invoc:path, $walk:ident) => {
        fn $visit(&mut self, node: &'a $ty) {
            if let $invoc(..) = node.node {
                self.visit_invoc(node.id);
            } else {
                visit::$walk(self, node);
            }
        }
    }
}

impl<'a, 'b> Visitor<'a> for BuildReducedGraphVisitor<'a, 'b> {
    method!(visit_impl_item: ast::ImplItem, ast::ImplItemKind::Macro, walk_impl_item);
    method!(visit_expr:      ast::Expr,     ast::ExprKind::Mac,       walk_expr);
    method!(visit_pat:       ast::Pat,      ast::PatKind::Mac,        walk_pat);
    method!(visit_ty:        ast::Ty,       ast::TyKind::Mac,         walk_ty);

    fn visit_item(&mut self, item: &'a Item) {
        let macro_use = match item.node {
            ItemKind::MacroDef(..) => {
                self.resolver.define_macro(item, self.expansion, &mut self.current_legacy_scope);
                return
            }
            ItemKind::Mac(..) => {
                self.current_legacy_scope = LegacyScope::Invocation(self.visit_invoc(item.id));
                return
            }
            ItemKind::Mod(..) => self.resolver.contains_macro_use(&item.attrs),
            _ => false,
        };

        let orig_current_module = self.resolver.current_module;
        let orig_current_legacy_scope = self.current_legacy_scope;
        let parent_scope = ParentScope {
            module: self.resolver.current_module,
            expansion: self.expansion,
            legacy: self.current_legacy_scope,
            derives: Vec::new(),
        };
        self.resolver.build_reduced_graph_for_item(item, parent_scope);
        visit::walk_item(self, item);
        self.resolver.current_module = orig_current_module;
        if !macro_use {
            self.current_legacy_scope = orig_current_legacy_scope;
        }
    }

    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        if let ast::StmtKind::Mac(..) = stmt.node {
            self.current_legacy_scope = LegacyScope::Invocation(self.visit_invoc(stmt.id));
        } else {
            visit::walk_stmt(self, stmt);
        }
    }

    fn visit_foreign_item(&mut self, foreign_item: &'a ForeignItem) {
        if let ForeignItemKind::Macro(_) = foreign_item.node {
            self.visit_invoc(foreign_item.id);
            return;
        }

        self.resolver.build_reduced_graph_for_foreign_item(foreign_item, self.expansion);
        visit::walk_foreign_item(self, foreign_item);
    }

    fn visit_block(&mut self, block: &'a Block) {
        let orig_current_module = self.resolver.current_module;
        let orig_current_legacy_scope = self.current_legacy_scope;
        self.resolver.build_reduced_graph_for_block(block, self.expansion);
        visit::walk_block(self, block);
        self.resolver.current_module = orig_current_module;
        self.current_legacy_scope = orig_current_legacy_scope;
    }

    fn visit_trait_item(&mut self, item: &'a TraitItem) {
        let parent = self.resolver.current_module;

        if let TraitItemKind::Macro(_) = item.node {
            self.visit_invoc(item.id);
            return
        }

        // Add the item to the trait info.
        let item_def_id = self.resolver.definitions.local_def_id(item.id);
        let (def, ns) = match item.node {
            TraitItemKind::Const(..) => (Def::AssociatedConst(item_def_id), ValueNS),
            TraitItemKind::Method(ref sig, _) => {
                if sig.decl.has_self() {
                    self.resolver.has_self.insert(item_def_id);
                }
                (Def::Method(item_def_id), ValueNS)
            }
            TraitItemKind::Type(..) => (Def::AssociatedTy(item_def_id), TypeNS),
            TraitItemKind::Macro(_) => bug!(),  // handled above
        };

        let vis = ty::Visibility::Public;
        self.resolver.define(parent, item.ident, ns, (def, vis, item.span, self.expansion));

        self.resolver.current_module = parent.parent.unwrap(); // nearest normal ancestor
        visit::walk_trait_item(self, item);
        self.resolver.current_module = parent;
    }

    fn visit_token(&mut self, t: Token) {
        if let Token::Interpolated(nt) = t {
            if let token::NtExpr(ref expr) = nt.0 {
                if let ast::ExprKind::Mac(..) = expr.node {
                    self.visit_invoc(expr.id);
                }
            }
        }
    }

    fn visit_attribute(&mut self, attr: &'a ast::Attribute) {
        if !attr.is_sugared_doc && is_builtin_attr(attr) {
            let parent_scope = ParentScope {
                module: self.resolver.current_module.nearest_item_scope(),
                expansion: self.expansion,
                legacy: self.current_legacy_scope,
                // Let's hope discerning built-in attributes from derive helpers is not necessary
                derives: Vec::new(),
            };
            parent_scope.module.builtin_attrs.borrow_mut().push((
                attr.path.segments[0].ident, parent_scope
            ));
        }
        visit::walk_attribute(self, attr);
    }
}
