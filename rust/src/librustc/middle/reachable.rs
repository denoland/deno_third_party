// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Finds items that are externally reachable, to determine which items
// need to have their metadata (and possibly their AST) serialized.
// All items that can be referred to through an exported name are
// reachable, and when a reachable thing is inline or generic, it
// makes all other generics or inline functions that it references
// reachable as well.

use hir::CodegenFnAttrs;
use hir::map as hir_map;
use hir::def::Def;
use hir::def_id::{DefId, CrateNum};
use rustc_data_structures::sync::Lrc;
use ty::{self, TyCtxt};
use ty::query::Providers;
use middle::privacy;
use session::config;
use util::nodemap::{NodeSet, FxHashSet};

use rustc_target::spec::abi::Abi;
use syntax::ast;
use syntax::attr;
use hir;
use hir::def_id::LOCAL_CRATE;
use hir::intravisit::{Visitor, NestedVisitorMap};
use hir::itemlikevisit::ItemLikeVisitor;
use hir::intravisit;

// Returns true if the given set of generics implies that the item it's
// associated with must be inlined.
fn generics_require_inlining(generics: &hir::Generics) -> bool {
    generics.params.iter().any(|param| param.is_type_param())
}

// Returns true if the given item must be inlined because it may be
// monomorphized or it was marked with `#[inline]`. This will only return
// true for functions.
fn item_might_be_inlined(item: &hir::Item, attrs: CodegenFnAttrs) -> bool {
    if attrs.requests_inline() {
        return true
    }

    match item.node {
        hir::ItemImpl(_, _, _, ref generics, ..) |
        hir::ItemFn(.., ref generics, _) => {
            generics_require_inlining(generics)
        }
        _ => false,
    }
}

fn method_might_be_inlined<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                     impl_item: &hir::ImplItem,
                                     impl_src: DefId) -> bool {
    let codegen_fn_attrs = tcx.codegen_fn_attrs(impl_item.hir_id.owner_def_id());
    if codegen_fn_attrs.requests_inline() ||
        generics_require_inlining(&impl_item.generics) {
        return true
    }
    if let Some(impl_node_id) = tcx.hir.as_local_node_id(impl_src) {
        match tcx.hir.find(impl_node_id) {
            Some(hir_map::NodeItem(item)) =>
                item_might_be_inlined(&item, codegen_fn_attrs),
            Some(..) | None =>
                span_bug!(impl_item.span, "impl did is not an item")
        }
    } else {
        span_bug!(impl_item.span, "found a foreign impl as a parent of a local method")
    }
}

// Information needed while computing reachability.
struct ReachableContext<'a, 'tcx: 'a> {
    // The type context.
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    tables: &'a ty::TypeckTables<'tcx>,
    // The set of items which must be exported in the linkage sense.
    reachable_symbols: NodeSet,
    // A worklist of item IDs. Each item ID in this worklist will be inlined
    // and will be scanned for further references.
    worklist: Vec<ast::NodeId>,
    // Whether any output of this compilation is a library
    any_library: bool,
}

impl<'a, 'tcx> Visitor<'tcx> for ReachableContext<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }

    fn visit_nested_body(&mut self, body: hir::BodyId) {
        let old_tables = self.tables;
        self.tables = self.tcx.body_tables(body);
        let body = self.tcx.hir.body(body);
        self.visit_body(body);
        self.tables = old_tables;
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr) {
        let def = match expr.node {
            hir::ExprPath(ref qpath) => {
                Some(self.tables.qpath_def(qpath, expr.hir_id))
            }
            hir::ExprMethodCall(..) => {
                Some(self.tables.type_dependent_defs()[expr.hir_id])
            }
            _ => None
        };

        match def {
            Some(Def::Local(node_id)) | Some(Def::Upvar(node_id, ..)) => {
                self.reachable_symbols.insert(node_id);
            }
            Some(def) => {
                let def_id = def.def_id();
                if let Some(node_id) = self.tcx.hir.as_local_node_id(def_id) {
                    if self.def_id_represents_local_inlined_item(def_id) {
                        self.worklist.push(node_id);
                    } else {
                        match def {
                            // If this path leads to a constant, then we need to
                            // recurse into the constant to continue finding
                            // items that are reachable.
                            Def::Const(..) | Def::AssociatedConst(..) => {
                                self.worklist.push(node_id);
                            }

                            // If this wasn't a static, then the destination is
                            // surely reachable.
                            _ => {
                                self.reachable_symbols.insert(node_id);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        intravisit::walk_expr(self, expr)
    }
}

impl<'a, 'tcx> ReachableContext<'a, 'tcx> {
    // Returns true if the given def ID represents a local item that is
    // eligible for inlining and false otherwise.
    fn def_id_represents_local_inlined_item(&self, def_id: DefId) -> bool {
        let node_id = match self.tcx.hir.as_local_node_id(def_id) {
            Some(node_id) => node_id,
            None => { return false; }
        };

        match self.tcx.hir.find(node_id) {
            Some(hir_map::NodeItem(item)) => {
                match item.node {
                    hir::ItemFn(..) =>
                        item_might_be_inlined(&item, self.tcx.codegen_fn_attrs(def_id)),
                    _ => false,
                }
            }
            Some(hir_map::NodeTraitItem(trait_method)) => {
                match trait_method.node {
                    hir::TraitItemKind::Const(_, ref default) => default.is_some(),
                    hir::TraitItemKind::Method(_, hir::TraitMethod::Provided(_)) => true,
                    hir::TraitItemKind::Method(_, hir::TraitMethod::Required(_)) |
                    hir::TraitItemKind::Type(..) => false,
                }
            }
            Some(hir_map::NodeImplItem(impl_item)) => {
                match impl_item.node {
                    hir::ImplItemKind::Const(..) => true,
                    hir::ImplItemKind::Method(..) => {
                        let attrs = self.tcx.codegen_fn_attrs(def_id);
                        if generics_require_inlining(&impl_item.generics) ||
                                attrs.requests_inline() {
                            true
                        } else {
                            let impl_did = self.tcx
                                               .hir
                                               .get_parent_did(node_id);
                            // Check the impl. If the generics on the self
                            // type of the impl require inlining, this method
                            // does too.
                            let impl_node_id = self.tcx.hir.as_local_node_id(impl_did).unwrap();
                            match self.tcx.hir.expect_item(impl_node_id).node {
                                hir::ItemImpl(_, _, _, ref generics, ..) => {
                                    generics_require_inlining(generics)
                                }
                                _ => false
                            }
                        }
                    }
                    hir::ImplItemKind::Type(_) => false,
                }
            }
            Some(_) => false,
            None => false   // This will happen for default methods.
        }
    }

    // Step 2: Mark all symbols that the symbols on the worklist touch.
    fn propagate(&mut self) {
        let mut scanned = FxHashSet();
        while let Some(search_item) = self.worklist.pop() {
            if !scanned.insert(search_item) {
                continue
            }

            if let Some(ref item) = self.tcx.hir.find(search_item) {
                self.propagate_node(item, search_item);
            }
        }
    }

    fn propagate_node(&mut self, node: &hir_map::Node<'tcx>,
                      search_item: ast::NodeId) {
        if !self.any_library {
            // If we are building an executable, only explicitly extern
            // types need to be exported.
            if let hir_map::NodeItem(item) = *node {
                let reachable = if let hir::ItemFn(.., abi, _, _) = item.node {
                    abi != Abi::Rust
                } else {
                    false
                };
                let def_id = self.tcx.hir.local_def_id(item.id);
                let is_extern = self.tcx.codegen_fn_attrs(def_id).contains_extern_indicator();
                if reachable || is_extern {
                    self.reachable_symbols.insert(search_item);
                }
            }
        } else {
            // If we are building a library, then reachable symbols will
            // continue to participate in linkage after this product is
            // produced. In this case, we traverse the ast node, recursing on
            // all reachable nodes from this one.
            self.reachable_symbols.insert(search_item);
        }

        match *node {
            hir_map::NodeItem(item) => {
                match item.node {
                    hir::ItemFn(.., body) => {
                        let def_id = self.tcx.hir.local_def_id(item.id);
                        if item_might_be_inlined(&item, self.tcx.codegen_fn_attrs(def_id)) {
                            self.visit_nested_body(body);
                        }
                    }

                    // Reachable constants will be inlined into other crates
                    // unconditionally, so we need to make sure that their
                    // contents are also reachable.
                    hir::ItemConst(_, init) => {
                        self.visit_nested_body(init);
                    }

                    // These are normal, nothing reachable about these
                    // inherently and their children are already in the
                    // worklist, as determined by the privacy pass
                    hir::ItemExternCrate(_) | hir::ItemUse(..) |
                    hir::ItemExistential(..) |
                    hir::ItemTy(..) | hir::ItemStatic(..) |
                    hir::ItemMod(..) | hir::ItemForeignMod(..) |
                    hir::ItemImpl(..) | hir::ItemTrait(..) | hir::ItemTraitAlias(..) |
                    hir::ItemStruct(..) | hir::ItemEnum(..) |
                    hir::ItemUnion(..) |  hir::ItemGlobalAsm(..) => {}
                }
            }
            hir_map::NodeTraitItem(trait_method) => {
                match trait_method.node {
                    hir::TraitItemKind::Const(_, None) |
                    hir::TraitItemKind::Method(_, hir::TraitMethod::Required(_)) => {
                        // Keep going, nothing to get exported
                    }
                    hir::TraitItemKind::Const(_, Some(body_id)) |
                    hir::TraitItemKind::Method(_, hir::TraitMethod::Provided(body_id)) => {
                        self.visit_nested_body(body_id);
                    }
                    hir::TraitItemKind::Type(..) => {}
                }
            }
            hir_map::NodeImplItem(impl_item) => {
                match impl_item.node {
                    hir::ImplItemKind::Const(_, body) => {
                        self.visit_nested_body(body);
                    }
                    hir::ImplItemKind::Method(_, body) => {
                        let did = self.tcx.hir.get_parent_did(search_item);
                        if method_might_be_inlined(self.tcx, impl_item, did) {
                            self.visit_nested_body(body)
                        }
                    }
                    hir::ImplItemKind::Type(_) => {}
                }
            }
            hir_map::NodeExpr(&hir::Expr { node: hir::ExprClosure(.., body, _, _), .. }) => {
                self.visit_nested_body(body);
            }
            // Nothing to recurse on for these
            hir_map::NodeForeignItem(_) |
            hir_map::NodeVariant(_) |
            hir_map::NodeStructCtor(_) |
            hir_map::NodeField(_) |
            hir_map::NodeTy(_) |
            hir_map::NodeMacroDef(_) => {}
            _ => {
                bug!("found unexpected thingy in worklist: {}",
                     self.tcx.hir.node_to_string(search_item))
            }
        }
    }
}

// Some methods from non-exported (completely private) trait impls still have to be
// reachable if they are called from inlinable code. Generally, it's not known until
// monomorphization if a specific trait impl item can be reachable or not. So, we
// conservatively mark all of them as reachable.
// FIXME: One possible strategy for pruning the reachable set is to avoid marking impl
// items of non-exported traits (or maybe all local traits?) unless their respective
// trait items are used from inlinable code through method call syntax or UFCS, or their
// trait is a lang item.
struct CollectPrivateImplItemsVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    access_levels: &'a privacy::AccessLevels,
    worklist: &'a mut Vec<ast::NodeId>,
}

impl<'a, 'tcx: 'a> ItemLikeVisitor<'tcx> for CollectPrivateImplItemsVisitor<'a, 'tcx> {
    fn visit_item(&mut self, item: &hir::Item) {
        // Anything which has custom linkage gets thrown on the worklist no
        // matter where it is in the crate.
        if attr::contains_name(&item.attrs, "linkage") {
            self.worklist.push(item.id);
        }

        // We need only trait impls here, not inherent impls, and only non-exported ones
        if let hir::ItemImpl(.., Some(ref trait_ref), _, ref impl_item_refs) = item.node {
            if !self.access_levels.is_reachable(item.id) {
                for impl_item_ref in impl_item_refs {
                    self.worklist.push(impl_item_ref.id.node_id);
                }

                let trait_def_id = match trait_ref.path.def {
                    Def::Trait(def_id) => def_id,
                    _ => unreachable!()
                };

                if !trait_def_id.is_local() {
                    return
                }

                for default_method in self.tcx.provided_trait_methods(trait_def_id) {
                    let node_id = self.tcx
                                      .hir
                                      .as_local_node_id(default_method.def_id)
                                      .unwrap();
                    self.worklist.push(node_id);
                }
            }
        }
    }

    fn visit_trait_item(&mut self, _trait_item: &hir::TraitItem) {}

    fn visit_impl_item(&mut self, _impl_item: &hir::ImplItem) {
        // processed in visit_item above
    }
}

// We introduce a new-type here, so we can have a specialized HashStable
// implementation for it.
#[derive(Clone)]
pub struct ReachableSet(pub Lrc<NodeSet>);


fn reachable_set<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, crate_num: CrateNum) -> ReachableSet {
    debug_assert!(crate_num == LOCAL_CRATE);

    let access_levels = &tcx.privacy_access_levels(LOCAL_CRATE);

    let any_library = tcx.sess.crate_types.borrow().iter().any(|ty| {
        *ty == config::CrateTypeRlib || *ty == config::CrateTypeDylib ||
        *ty == config::CrateTypeProcMacro
    });
    let mut reachable_context = ReachableContext {
        tcx,
        tables: &ty::TypeckTables::empty(None),
        reachable_symbols: NodeSet(),
        worklist: Vec::new(),
        any_library,
    };

    // Step 1: Seed the worklist with all nodes which were found to be public as
    //         a result of the privacy pass along with all local lang items and impl items.
    //         If other crates link to us, they're going to expect to be able to
    //         use the lang items, so we need to be sure to mark them as
    //         exported.
    for (id, _) in &access_levels.map {
        reachable_context.worklist.push(*id);
    }
    for item in tcx.lang_items().items().iter() {
        if let Some(did) = *item {
            if let Some(node_id) = tcx.hir.as_local_node_id(did) {
                reachable_context.worklist.push(node_id);
            }
        }
    }
    {
        let mut collect_private_impl_items = CollectPrivateImplItemsVisitor {
            tcx,
            access_levels,
            worklist: &mut reachable_context.worklist,
        };
        tcx.hir.krate().visit_all_item_likes(&mut collect_private_impl_items);
    }

    // Step 2: Mark all symbols that the symbols on the worklist touch.
    reachable_context.propagate();

    // Return the set of reachable symbols.
    ReachableSet(Lrc::new(reachable_context.reachable_symbols))
}

pub fn provide(providers: &mut Providers) {
    *providers = Providers {
        reachable_set,
        ..*providers
    };
}
