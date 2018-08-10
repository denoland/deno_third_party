// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module contains the "cleaned" pieces of the AST, and the functions
//! that clean them.

pub use self::Type::*;
pub use self::Mutability::*;
pub use self::ItemEnum::*;
pub use self::TyParamBound::*;
pub use self::SelfTy::*;
pub use self::FunctionRetTy::*;
pub use self::Visibility::{Public, Inherited};

use syntax;
use rustc_target::spec::abi::Abi;
use syntax::ast::{self, AttrStyle, NodeId, Ident};
use syntax::attr;
use syntax::codemap::{dummy_spanned, Spanned};
use syntax::feature_gate::UnstableFeatures;
use syntax::ptr::P;
use syntax::symbol::keywords::{self, Keyword};
use syntax::symbol::{Symbol, InternedString};
use syntax_pos::{self, DUMMY_SP, Pos, FileName};

use rustc::middle::const_val::ConstVal;
use rustc::middle::privacy::AccessLevels;
use rustc::middle::resolve_lifetime as rl;
use rustc::ty::fold::TypeFolder;
use rustc::middle::lang_items;
use rustc::mir::interpret::GlobalId;
use rustc::hir::{self, HirVec};
use rustc::hir::def::{self, Def, CtorKind};
use rustc::hir::def_id::{CrateNum, DefId, DefIndex, CRATE_DEF_INDEX, LOCAL_CRATE};
use rustc::hir::def_id::DefIndexAddressSpace;
use rustc::ty::subst::Substs;
use rustc::ty::{self, TyCtxt, Region, RegionVid, Ty, AdtKind, GenericParamCount};
use rustc::middle::stability;
use rustc::util::nodemap::{FxHashMap, FxHashSet};
use rustc_typeck::hir_ty_to_ty;
use rustc::infer::region_constraints::{RegionConstraintData, Constraint};
use rustc::lint as lint;

use std::collections::hash_map::Entry;
use std::fmt;
use std::default::Default;
use std::{mem, slice, vec};
use std::iter::{FromIterator, once};
use rustc_data_structures::sync::Lrc;
use std::rc::Rc;
use std::str::FromStr;
use std::cell::RefCell;
use std::sync::Arc;
use std::u32;
use std::ops::Range;

use core::{self, DocContext};
use doctree;
use visit_ast;
use html::render::{cache, ExternalLocation};
use html::item_type::ItemType;
use html::markdown::markdown_links;

pub mod inline;
pub mod cfg;
mod simplify;
mod auto_trait;

use self::cfg::Cfg;
use self::auto_trait::AutoTraitFinder;

thread_local!(static MAX_DEF_ID: RefCell<FxHashMap<CrateNum, DefId>> = RefCell::new(FxHashMap()));

const FN_OUTPUT_NAME: &'static str = "Output";

// extract the stability index for a node from tcx, if possible
fn get_stability(cx: &DocContext, def_id: DefId) -> Option<Stability> {
    cx.tcx.lookup_stability(def_id).clean(cx)
}

fn get_deprecation(cx: &DocContext, def_id: DefId) -> Option<Deprecation> {
    cx.tcx.lookup_deprecation(def_id).clean(cx)
}

pub trait Clean<T> {
    fn clean(&self, cx: &DocContext) -> T;
}

impl<T: Clean<U>, U> Clean<Vec<U>> for [T] {
    fn clean(&self, cx: &DocContext) -> Vec<U> {
        self.iter().map(|x| x.clean(cx)).collect()
    }
}

impl<T: Clean<U>, U> Clean<U> for P<T> {
    fn clean(&self, cx: &DocContext) -> U {
        (**self).clean(cx)
    }
}

impl<T: Clean<U>, U> Clean<U> for Rc<T> {
    fn clean(&self, cx: &DocContext) -> U {
        (**self).clean(cx)
    }
}

impl<T: Clean<U>, U> Clean<Option<U>> for Option<T> {
    fn clean(&self, cx: &DocContext) -> Option<U> {
        self.as_ref().map(|v| v.clean(cx))
    }
}

impl<T, U> Clean<U> for ty::Binder<T> where T: Clean<U> {
    fn clean(&self, cx: &DocContext) -> U {
        self.skip_binder().clean(cx)
    }
}

impl<T: Clean<U>, U> Clean<Vec<U>> for P<[T]> {
    fn clean(&self, cx: &DocContext) -> Vec<U> {
        self.iter().map(|x| x.clean(cx)).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Crate {
    pub name: String,
    pub version: Option<String>,
    pub src: FileName,
    pub module: Option<Item>,
    pub externs: Vec<(CrateNum, ExternalCrate)>,
    pub primitives: Vec<(DefId, PrimitiveType, Attributes)>,
    pub access_levels: Arc<AccessLevels<DefId>>,
    // These are later on moved into `CACHEKEY`, leaving the map empty.
    // Only here so that they can be filtered through the rustdoc passes.
    pub external_traits: FxHashMap<DefId, Trait>,
    pub masked_crates: FxHashSet<CrateNum>,
}

impl<'a, 'tcx, 'rcx> Clean<Crate> for visit_ast::RustdocVisitor<'a, 'tcx, 'rcx> {
    fn clean(&self, cx: &DocContext) -> Crate {
        use ::visit_lib::LibEmbargoVisitor;

        {
            let mut r = cx.renderinfo.borrow_mut();
            r.deref_trait_did = cx.tcx.lang_items().deref_trait();
            r.deref_mut_trait_did = cx.tcx.lang_items().deref_mut_trait();
            r.owned_box_did = cx.tcx.lang_items().owned_box();
        }

        let mut externs = Vec::new();
        for &cnum in cx.tcx.crates().iter() {
            externs.push((cnum, cnum.clean(cx)));
            // Analyze doc-reachability for extern items
            LibEmbargoVisitor::new(cx).visit_lib(cnum);
        }
        externs.sort_by(|&(a, _), &(b, _)| a.cmp(&b));

        // Clean the crate, translating the entire libsyntax AST to one that is
        // understood by rustdoc.
        let mut module = self.module.clean(cx);
        let mut masked_crates = FxHashSet();

        match module.inner {
            ModuleItem(ref module) => {
                for it in &module.items {
                    if it.is_extern_crate() && it.attrs.has_doc_flag("masked") {
                        masked_crates.insert(it.def_id.krate);
                    }
                }
            }
            _ => unreachable!(),
        }

        let ExternalCrate { name, src, primitives, keywords, .. } = LOCAL_CRATE.clean(cx);
        {
            let m = match module.inner {
                ModuleItem(ref mut m) => m,
                _ => unreachable!(),
            };
            m.items.extend(primitives.iter().map(|&(def_id, prim, ref attrs)| {
                Item {
                    source: Span::empty(),
                    name: Some(prim.to_url_str().to_string()),
                    attrs: attrs.clone(),
                    visibility: Some(Public),
                    stability: get_stability(cx, def_id),
                    deprecation: get_deprecation(cx, def_id),
                    def_id,
                    inner: PrimitiveItem(prim),
                }
            }));
            m.items.extend(keywords.into_iter().map(|(def_id, kw, attrs)| {
                Item {
                    source: Span::empty(),
                    name: Some(kw.clone()),
                    attrs: attrs,
                    visibility: Some(Public),
                    stability: get_stability(cx, def_id),
                    deprecation: get_deprecation(cx, def_id),
                    def_id,
                    inner: KeywordItem(kw),
                }
            }));
        }

        let mut access_levels = cx.access_levels.borrow_mut();
        let mut external_traits = cx.external_traits.borrow_mut();

        Crate {
            name,
            version: None,
            src,
            module: Some(module),
            externs,
            primitives,
            access_levels: Arc::new(mem::replace(&mut access_levels, Default::default())),
            external_traits: mem::replace(&mut external_traits, Default::default()),
            masked_crates,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct ExternalCrate {
    pub name: String,
    pub src: FileName,
    pub attrs: Attributes,
    pub primitives: Vec<(DefId, PrimitiveType, Attributes)>,
    pub keywords: Vec<(DefId, String, Attributes)>,
}

impl Clean<ExternalCrate> for CrateNum {
    fn clean(&self, cx: &DocContext) -> ExternalCrate {
        let root = DefId { krate: *self, index: CRATE_DEF_INDEX };
        let krate_span = cx.tcx.def_span(root);
        let krate_src = cx.sess().codemap().span_to_filename(krate_span);

        // Collect all inner modules which are tagged as implementations of
        // primitives.
        //
        // Note that this loop only searches the top-level items of the crate,
        // and this is intentional. If we were to search the entire crate for an
        // item tagged with `#[doc(primitive)]` then we would also have to
        // search the entirety of external modules for items tagged
        // `#[doc(primitive)]`, which is a pretty inefficient process (decoding
        // all that metadata unconditionally).
        //
        // In order to keep the metadata load under control, the
        // `#[doc(primitive)]` feature is explicitly designed to only allow the
        // primitive tags to show up as the top level items in a crate.
        //
        // Also note that this does not attempt to deal with modules tagged
        // duplicately for the same primitive. This is handled later on when
        // rendering by delegating everything to a hash map.
        let as_primitive = |def: Def| {
            if let Def::Mod(def_id) = def {
                let attrs = cx.tcx.get_attrs(def_id).clean(cx);
                let mut prim = None;
                for attr in attrs.lists("doc") {
                    if let Some(v) = attr.value_str() {
                        if attr.check_name("primitive") {
                            prim = PrimitiveType::from_str(&v.as_str());
                            if prim.is_some() {
                                break;
                            }
                            // FIXME: should warn on unknown primitives?
                        }
                    }
                }
                return prim.map(|p| (def_id, p, attrs));
            }
            None
        };
        let primitives = if root.is_local() {
            cx.tcx.hir.krate().module.item_ids.iter().filter_map(|&id| {
                let item = cx.tcx.hir.expect_item(id.id);
                match item.node {
                    hir::ItemMod(_) => {
                        as_primitive(Def::Mod(cx.tcx.hir.local_def_id(id.id)))
                    }
                    hir::ItemUse(ref path, hir::UseKind::Single)
                    if item.vis == hir::Visibility::Public => {
                        as_primitive(path.def).map(|(_, prim, attrs)| {
                            // Pretend the primitive is local.
                            (cx.tcx.hir.local_def_id(id.id), prim, attrs)
                        })
                    }
                    _ => None
                }
            }).collect()
        } else {
            cx.tcx.item_children(root).iter().map(|item| item.def)
              .filter_map(as_primitive).collect()
        };

        let as_keyword = |def: Def| {
            if let Def::Mod(def_id) = def {
                let attrs = cx.tcx.get_attrs(def_id).clean(cx);
                let mut keyword = None;
                for attr in attrs.lists("doc") {
                    if let Some(v) = attr.value_str() {
                        if attr.check_name("keyword") {
                            keyword = Keyword::from_str(&v.as_str()).ok()
                                                                    .map(|x| x.name().to_string());
                            if keyword.is_some() {
                                break
                            }
                            // FIXME: should warn on unknown keywords?
                        }
                    }
                }
                return keyword.map(|p| (def_id, p, attrs));
            }
            None
        };
        let keywords = if root.is_local() {
            cx.tcx.hir.krate().module.item_ids.iter().filter_map(|&id| {
                let item = cx.tcx.hir.expect_item(id.id);
                match item.node {
                    hir::ItemMod(_) => {
                        as_keyword(Def::Mod(cx.tcx.hir.local_def_id(id.id)))
                    }
                    hir::ItemUse(ref path, hir::UseKind::Single)
                    if item.vis == hir::Visibility::Public => {
                        as_keyword(path.def).map(|(_, prim, attrs)| {
                            (cx.tcx.hir.local_def_id(id.id), prim, attrs)
                        })
                    }
                    _ => None
                }
            }).collect()
        } else {
            cx.tcx.item_children(root).iter().map(|item| item.def)
              .filter_map(as_keyword).collect()
        };

        ExternalCrate {
            name: cx.tcx.crate_name(*self).to_string(),
            src: krate_src,
            attrs: cx.tcx.get_attrs(root).clean(cx),
            primitives,
            keywords,
        }
    }
}

/// Anything with a source location and set of attributes and, optionally, a
/// name. That is, anything that can be documented. This doesn't correspond
/// directly to the AST's concept of an item; it's a strict superset.
#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Item {
    /// Stringified span
    pub source: Span,
    /// Not everything has a name. E.g., impls
    pub name: Option<String>,
    pub attrs: Attributes,
    pub inner: ItemEnum,
    pub visibility: Option<Visibility>,
    pub def_id: DefId,
    pub stability: Option<Stability>,
    pub deprecation: Option<Deprecation>,
}

impl fmt::Debug for Item {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {

        let fake = MAX_DEF_ID.with(|m| m.borrow().get(&self.def_id.krate)
                                   .map(|id| self.def_id >= *id).unwrap_or(false));
        let def_id: &fmt::Debug = if fake { &"**FAKE**" } else { &self.def_id };

        fmt.debug_struct("Item")
            .field("source", &self.source)
            .field("name", &self.name)
            .field("attrs", &self.attrs)
            .field("inner", &self.inner)
            .field("visibility", &self.visibility)
            .field("def_id", def_id)
            .field("stability", &self.stability)
            .field("deprecation", &self.deprecation)
            .finish()
    }
}

impl Item {
    /// Finds the `doc` attribute as a NameValue and returns the corresponding
    /// value found.
    pub fn doc_value<'a>(&'a self) -> Option<&'a str> {
        self.attrs.doc_value()
    }
    /// Finds all `doc` attributes as NameValues and returns their corresponding values, joined
    /// with newlines.
    pub fn collapsed_doc_value(&self) -> Option<String> {
        self.attrs.collapsed_doc_value()
    }

    pub fn links(&self) -> Vec<(String, String)> {
        self.attrs.links(&self.def_id.krate)
    }

    pub fn is_crate(&self) -> bool {
        match self.inner {
            StrippedItem(box ModuleItem(Module { is_crate: true, ..})) |
            ModuleItem(Module { is_crate: true, ..}) => true,
            _ => false,
        }
    }
    pub fn is_mod(&self) -> bool {
        self.type_() == ItemType::Module
    }
    pub fn is_trait(&self) -> bool {
        self.type_() == ItemType::Trait
    }
    pub fn is_struct(&self) -> bool {
        self.type_() == ItemType::Struct
    }
    pub fn is_enum(&self) -> bool {
        self.type_() == ItemType::Enum
    }
    pub fn is_fn(&self) -> bool {
        self.type_() == ItemType::Function
    }
    pub fn is_associated_type(&self) -> bool {
        self.type_() == ItemType::AssociatedType
    }
    pub fn is_associated_const(&self) -> bool {
        self.type_() == ItemType::AssociatedConst
    }
    pub fn is_method(&self) -> bool {
        self.type_() == ItemType::Method
    }
    pub fn is_ty_method(&self) -> bool {
        self.type_() == ItemType::TyMethod
    }
    pub fn is_typedef(&self) -> bool {
        self.type_() == ItemType::Typedef
    }
    pub fn is_primitive(&self) -> bool {
        self.type_() == ItemType::Primitive
    }
    pub fn is_union(&self) -> bool {
        self.type_() == ItemType::Union
    }
    pub fn is_import(&self) -> bool {
        self.type_() == ItemType::Import
    }
    pub fn is_extern_crate(&self) -> bool {
        self.type_() == ItemType::ExternCrate
    }
    pub fn is_keyword(&self) -> bool {
        self.type_() == ItemType::Keyword
    }

    pub fn is_stripped(&self) -> bool {
        match self.inner { StrippedItem(..) => true, _ => false }
    }
    pub fn has_stripped_fields(&self) -> Option<bool> {
        match self.inner {
            StructItem(ref _struct) => Some(_struct.fields_stripped),
            UnionItem(ref union) => Some(union.fields_stripped),
            VariantItem(Variant { kind: VariantKind::Struct(ref vstruct)} ) => {
                Some(vstruct.fields_stripped)
            },
            _ => None,
        }
    }

    pub fn stability_class(&self) -> Option<String> {
        self.stability.as_ref().and_then(|ref s| {
            let mut classes = Vec::with_capacity(2);

            if s.level == stability::Unstable {
                classes.push("unstable");
            }

            if !s.deprecated_since.is_empty() {
                classes.push("deprecated");
            }

            if classes.len() != 0 {
                Some(classes.join(" "))
            } else {
                None
            }
        })
    }

    pub fn stable_since(&self) -> Option<&str> {
        self.stability.as_ref().map(|s| &s.since[..])
    }

    /// Returns a documentation-level item type from the item.
    pub fn type_(&self) -> ItemType {
        ItemType::from(self)
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub enum ItemEnum {
    ExternCrateItem(String, Option<String>),
    ImportItem(Import),
    StructItem(Struct),
    UnionItem(Union),
    EnumItem(Enum),
    FunctionItem(Function),
    ModuleItem(Module),
    TypedefItem(Typedef, bool /* is associated type */),
    StaticItem(Static),
    ConstantItem(Constant),
    TraitItem(Trait),
    ImplItem(Impl),
    /// A method signature only. Used for required methods in traits (ie,
    /// non-default-methods).
    TyMethodItem(TyMethod),
    /// A method with a body.
    MethodItem(Method),
    StructFieldItem(Type),
    VariantItem(Variant),
    /// `fn`s from an extern block
    ForeignFunctionItem(Function),
    /// `static`s from an extern block
    ForeignStaticItem(Static),
    /// `type`s from an extern block
    ForeignTypeItem,
    MacroItem(Macro),
    PrimitiveItem(PrimitiveType),
    AssociatedConstItem(Type, Option<String>),
    AssociatedTypeItem(Vec<TyParamBound>, Option<Type>),
    /// An item that has been stripped by a rustdoc pass
    StrippedItem(Box<ItemEnum>),
    KeywordItem(String),
}

impl ItemEnum {
    pub fn generics(&self) -> Option<&Generics> {
        Some(match *self {
            ItemEnum::StructItem(ref s) => &s.generics,
            ItemEnum::EnumItem(ref e) => &e.generics,
            ItemEnum::FunctionItem(ref f) => &f.generics,
            ItemEnum::TypedefItem(ref t, _) => &t.generics,
            ItemEnum::TraitItem(ref t) => &t.generics,
            ItemEnum::ImplItem(ref i) => &i.generics,
            ItemEnum::TyMethodItem(ref i) => &i.generics,
            ItemEnum::MethodItem(ref i) => &i.generics,
            ItemEnum::ForeignFunctionItem(ref f) => &f.generics,
            _ => return None,
        })
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Module {
    pub items: Vec<Item>,
    pub is_crate: bool,
}

impl Clean<Item> for doctree::Module {
    fn clean(&self, cx: &DocContext) -> Item {
        let name = if self.name.is_some() {
            self.name.unwrap().clean(cx)
        } else {
            "".to_string()
        };

        // maintain a stack of mod ids, for doc comment path resolution
        // but we also need to resolve the module's own docs based on whether its docs were written
        // inside or outside the module, so check for that
        let attrs = if self.attrs.iter()
                                 .filter(|a| a.check_name("doc"))
                                 .next()
                                 .map_or(true, |a| a.style == AttrStyle::Inner) {
            // inner doc comment, use the module's own scope for resolution
            cx.mod_ids.borrow_mut().push(self.id);
            self.attrs.clean(cx)
        } else {
            // outer doc comment, use its parent's scope
            let attrs = self.attrs.clean(cx);
            cx.mod_ids.borrow_mut().push(self.id);
            attrs
        };

        let mut items: Vec<Item> = vec![];
        items.extend(self.extern_crates.iter().map(|x| x.clean(cx)));
        items.extend(self.imports.iter().flat_map(|x| x.clean(cx)));
        items.extend(self.structs.iter().flat_map(|x| x.clean(cx)));
        items.extend(self.unions.iter().flat_map(|x| x.clean(cx)));
        items.extend(self.enums.iter().flat_map(|x| x.clean(cx)));
        items.extend(self.fns.iter().map(|x| x.clean(cx)));
        items.extend(self.foreigns.iter().flat_map(|x| x.clean(cx)));
        items.extend(self.mods.iter().map(|x| x.clean(cx)));
        items.extend(self.typedefs.iter().map(|x| x.clean(cx)));
        items.extend(self.statics.iter().map(|x| x.clean(cx)));
        items.extend(self.constants.iter().map(|x| x.clean(cx)));
        items.extend(self.traits.iter().map(|x| x.clean(cx)));
        items.extend(self.impls.iter().flat_map(|x| x.clean(cx)));
        items.extend(self.macros.iter().map(|x| x.clean(cx)));

        cx.mod_ids.borrow_mut().pop();

        // determine if we should display the inner contents or
        // the outer `mod` item for the source code.
        let whence = {
            let cm = cx.sess().codemap();
            let outer = cm.lookup_char_pos(self.where_outer.lo());
            let inner = cm.lookup_char_pos(self.where_inner.lo());
            if outer.file.start_pos == inner.file.start_pos {
                // mod foo { ... }
                self.where_outer
            } else {
                // mod foo; (and a separate FileMap for the contents)
                self.where_inner
            }
        };

        Item {
            name: Some(name),
            attrs,
            source: whence.clean(cx),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            inner: ModuleItem(Module {
               is_crate: self.is_crate,
               items,
            })
        }
    }
}

pub struct ListAttributesIter<'a> {
    attrs: slice::Iter<'a, ast::Attribute>,
    current_list: vec::IntoIter<ast::NestedMetaItem>,
    name: &'a str
}

impl<'a> Iterator for ListAttributesIter<'a> {
    type Item = ast::NestedMetaItem;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(nested) = self.current_list.next() {
            return Some(nested);
        }

        for attr in &mut self.attrs {
            if let Some(list) = attr.meta_item_list() {
                if attr.check_name(self.name) {
                    self.current_list = list.into_iter();
                    if let Some(nested) = self.current_list.next() {
                        return Some(nested);
                    }
                }
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let lower = self.current_list.len();
        (lower, None)
    }
}

pub trait AttributesExt {
    /// Finds an attribute as List and returns the list of attributes nested inside.
    fn lists<'a>(&'a self, name: &'a str) -> ListAttributesIter<'a>;
}

impl AttributesExt for [ast::Attribute] {
    fn lists<'a>(&'a self, name: &'a str) -> ListAttributesIter<'a> {
        ListAttributesIter {
            attrs: self.iter(),
            current_list: Vec::new().into_iter(),
            name,
        }
    }
}

pub trait NestedAttributesExt {
    /// Returns whether the attribute list contains a specific `Word`
    fn has_word(self, word: &str) -> bool;
}

impl<I: IntoIterator<Item=ast::NestedMetaItem>> NestedAttributesExt for I {
    fn has_word(self, word: &str) -> bool {
        self.into_iter().any(|attr| attr.is_word() && attr.check_name(word))
    }
}

/// A portion of documentation, extracted from a `#[doc]` attribute.
///
/// Each variant contains the line number within the complete doc-comment where the fragment
/// starts, as well as the Span where the corresponding doc comment or attribute is located.
///
/// Included files are kept separate from inline doc comments so that proper line-number
/// information can be given when a doctest fails. Sugared doc comments and "raw" doc comments are
/// kept separate because of issue #42760.
#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum DocFragment {
    // FIXME #44229 (misdreavus): sugared and raw doc comments can be brought back together once
    // hoedown is completely removed from rustdoc.
    /// A doc fragment created from a `///` or `//!` doc comment.
    SugaredDoc(usize, syntax_pos::Span, String),
    /// A doc fragment created from a "raw" `#[doc=""]` attribute.
    RawDoc(usize, syntax_pos::Span, String),
    /// A doc fragment created from a `#[doc(include="filename")]` attribute. Contains both the
    /// given filename and the file contents.
    Include(usize, syntax_pos::Span, String, String),
}

impl DocFragment {
    pub fn as_str(&self) -> &str {
        match *self {
            DocFragment::SugaredDoc(_, _, ref s) => &s[..],
            DocFragment::RawDoc(_, _, ref s) => &s[..],
            DocFragment::Include(_, _, _, ref s) => &s[..],
        }
    }

    pub fn span(&self) -> syntax_pos::Span {
        match *self {
            DocFragment::SugaredDoc(_, span, _) |
                DocFragment::RawDoc(_, span, _) |
                DocFragment::Include(_, span, _, _) => span,
        }
    }
}

impl<'a> FromIterator<&'a DocFragment> for String {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = &'a DocFragment>
    {
        iter.into_iter().fold(String::new(), |mut acc, frag| {
            if !acc.is_empty() {
                acc.push('\n');
            }
            match *frag {
                DocFragment::SugaredDoc(_, _, ref docs)
                    | DocFragment::RawDoc(_, _, ref docs)
                    | DocFragment::Include(_, _, _, ref docs) =>
                    acc.push_str(docs),
            }

            acc
        })
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Default, Hash)]
pub struct Attributes {
    pub doc_strings: Vec<DocFragment>,
    pub other_attrs: Vec<ast::Attribute>,
    pub cfg: Option<Arc<Cfg>>,
    pub span: Option<syntax_pos::Span>,
    /// map from Rust paths to resolved defs and potential URL fragments
    pub links: Vec<(String, Option<DefId>, Option<String>)>,
}

impl Attributes {
    /// Extracts the content from an attribute `#[doc(cfg(content))]`.
    fn extract_cfg(mi: &ast::MetaItem) -> Option<&ast::MetaItem> {
        use syntax::ast::NestedMetaItemKind::MetaItem;

        if let ast::MetaItemKind::List(ref nmis) = mi.node {
            if nmis.len() == 1 {
                if let MetaItem(ref cfg_mi) = nmis[0].node {
                    if cfg_mi.check_name("cfg") {
                        if let ast::MetaItemKind::List(ref cfg_nmis) = cfg_mi.node {
                            if cfg_nmis.len() == 1 {
                                if let MetaItem(ref content_mi) = cfg_nmis[0].node {
                                    return Some(content_mi);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Reads a `MetaItem` from within an attribute, looks for whether it is a
    /// `#[doc(include="file")]`, and returns the filename and contents of the file as loaded from
    /// its expansion.
    fn extract_include(mi: &ast::MetaItem)
        -> Option<(String, String)>
    {
        mi.meta_item_list().and_then(|list| {
            for meta in list {
                if meta.check_name("include") {
                    // the actual compiled `#[doc(include="filename")]` gets expanded to
                    // `#[doc(include(file="filename", contents="file contents")]` so we need to
                    // look for that instead
                    return meta.meta_item_list().and_then(|list| {
                        let mut filename: Option<String> = None;
                        let mut contents: Option<String> = None;

                        for it in list {
                            if it.check_name("file") {
                                if let Some(name) = it.value_str() {
                                    filename = Some(name.to_string());
                                }
                            } else if it.check_name("contents") {
                                if let Some(docs) = it.value_str() {
                                    contents = Some(docs.to_string());
                                }
                            }
                        }

                        if let (Some(filename), Some(contents)) = (filename, contents) {
                            Some((filename, contents))
                        } else {
                            None
                        }
                    });
                }
            }

            None
        })
    }

    pub fn has_doc_flag(&self, flag: &str) -> bool {
        for attr in &self.other_attrs {
            if !attr.check_name("doc") { continue; }

            if let Some(items) = attr.meta_item_list() {
                if items.iter().filter_map(|i| i.meta_item()).any(|it| it.check_name(flag)) {
                    return true;
                }
            }
        }

        false
    }

    pub fn from_ast(diagnostic: &::errors::Handler,
                    attrs: &[ast::Attribute]) -> Attributes {
        let mut doc_strings = vec![];
        let mut sp = None;
        let mut cfg = Cfg::True;
        let mut doc_line = 0;

        let other_attrs = attrs.iter().filter_map(|attr| {
            attr.with_desugared_doc(|attr| {
                if attr.check_name("doc") {
                    if let Some(mi) = attr.meta() {
                        if let Some(value) = mi.value_str() {
                            // Extracted #[doc = "..."]
                            let value = value.to_string();
                            let line = doc_line;
                            doc_line += value.lines().count();

                            if attr.is_sugared_doc {
                                doc_strings.push(DocFragment::SugaredDoc(line, attr.span, value));
                            } else {
                                doc_strings.push(DocFragment::RawDoc(line, attr.span, value));
                            }

                            if sp.is_none() {
                                sp = Some(attr.span);
                            }
                            return None;
                        } else if let Some(cfg_mi) = Attributes::extract_cfg(&mi) {
                            // Extracted #[doc(cfg(...))]
                            match Cfg::parse(cfg_mi) {
                                Ok(new_cfg) => cfg &= new_cfg,
                                Err(e) => diagnostic.span_err(e.span, e.msg),
                            }
                            return None;
                        } else if let Some((filename, contents)) = Attributes::extract_include(&mi)
                        {
                            let line = doc_line;
                            doc_line += contents.lines().count();
                            doc_strings.push(DocFragment::Include(line,
                                                                  attr.span,
                                                                  filename,
                                                                  contents));
                        }
                    }
                }
                Some(attr.clone())
            })
        }).collect();

        // treat #[target_feature(enable = "feat")] attributes as if they were
        // #[doc(cfg(target_feature = "feat"))] attributes as well
        for attr in attrs.lists("target_feature") {
            if attr.check_name("enable") {
                if let Some(feat) = attr.value_str() {
                    let meta = attr::mk_name_value_item_str(Ident::from_str("target_feature"),
                                                            dummy_spanned(feat));
                    if let Ok(feat_cfg) = Cfg::parse(&meta) {
                        cfg &= feat_cfg;
                    }
                }
            }
        }

        Attributes {
            doc_strings,
            other_attrs,
            cfg: if cfg == Cfg::True { None } else { Some(Arc::new(cfg)) },
            span: sp,
            links: vec![],
        }
    }

    /// Finds the `doc` attribute as a NameValue and returns the corresponding
    /// value found.
    pub fn doc_value<'a>(&'a self) -> Option<&'a str> {
        self.doc_strings.first().map(|s| s.as_str())
    }

    /// Finds all `doc` attributes as NameValues and returns their corresponding values, joined
    /// with newlines.
    pub fn collapsed_doc_value(&self) -> Option<String> {
        if !self.doc_strings.is_empty() {
            Some(self.doc_strings.iter().collect())
        } else {
            None
        }
    }

    /// Get links as a vector
    ///
    /// Cache must be populated before call
    pub fn links(&self, krate: &CrateNum) -> Vec<(String, String)> {
        use html::format::href;
        self.links.iter().filter_map(|&(ref s, did, ref fragment)| {
            match did {
                Some(did) => {
                    if let Some((mut href, ..)) = href(did) {
                        if let Some(ref fragment) = *fragment {
                            href.push_str("#");
                            href.push_str(fragment);
                        }
                        Some((s.clone(), href))
                    } else {
                        None
                    }
                }
                None => {
                    if let Some(ref fragment) = *fragment {
                        let cache = cache();
                        let url = match cache.extern_locations.get(krate) {
                            Some(&(_, ref src, ExternalLocation::Local)) =>
                                src.to_str().expect("invalid file path"),
                            Some(&(_, _, ExternalLocation::Remote(ref s))) => s,
                            Some(&(_, _, ExternalLocation::Unknown)) | None =>
                                "https://doc.rust-lang.org/nightly",
                        };
                        // This is a primitive so the url is done "by hand".
                        Some((s.clone(),
                              format!("{}{}std/primitive.{}.html",
                                      url,
                                      if !url.ends_with('/') { "/" } else { "" },
                                      fragment)))
                    } else {
                        panic!("This isn't a primitive?!");
                    }
                }
            }
        }).collect()
    }
}

impl AttributesExt for Attributes {
    fn lists<'a>(&'a self, name: &'a str) -> ListAttributesIter<'a> {
        self.other_attrs.lists(name)
    }
}

/// Given a def, returns its name and disambiguator
/// for a value namespace
///
/// Returns None for things which cannot be ambiguous since
/// they exist in both namespaces (structs and modules)
fn value_ns_kind(def: Def, path_str: &str) -> Option<(&'static str, String)> {
    match def {
        // structs, variants, and mods exist in both namespaces. skip them
        Def::StructCtor(..) | Def::Mod(..) | Def::Variant(..) | Def::VariantCtor(..) => None,
        Def::Fn(..)
            => Some(("function", format!("{}()", path_str))),
        Def::Method(..)
            => Some(("method", format!("{}()", path_str))),
        Def::Const(..)
            => Some(("const", format!("const@{}", path_str))),
        Def::Static(..)
            => Some(("static", format!("static@{}", path_str))),
        _ => Some(("value", format!("value@{}", path_str))),
    }
}

/// Given a def, returns its name, the article to be used, and a disambiguator
/// for the type namespace
fn type_ns_kind(def: Def, path_str: &str) -> (&'static str, &'static str, String) {
    let (kind, article) = match def {
        // we can still have non-tuple structs
        Def::Struct(..) => ("struct", "a"),
        Def::Enum(..) => ("enum", "an"),
        Def::Trait(..) => ("trait", "a"),
        Def::Union(..) => ("union", "a"),
        _ => ("type", "a"),
    };
    (kind, article, format!("{}@{}", kind, path_str))
}

fn span_of_attrs(attrs: &Attributes) -> syntax_pos::Span {
    if attrs.doc_strings.is_empty() {
        return DUMMY_SP;
    }
    let start = attrs.doc_strings[0].span();
    let end = attrs.doc_strings.last().unwrap().span();
    start.to(end)
}

fn ambiguity_error(cx: &DocContext, attrs: &Attributes,
                   path_str: &str,
                   article1: &str, kind1: &str, disambig1: &str,
                   article2: &str, kind2: &str, disambig2: &str) {
    let sp = span_of_attrs(attrs);
    cx.sess()
      .struct_span_warn(sp,
                        &format!("`{}` is both {} {} and {} {}",
                                 path_str, article1, kind1,
                                 article2, kind2))
      .help(&format!("try `{}` if you want to select the {}, \
                      or `{}` if you want to \
                      select the {}",
                      disambig1, kind1, disambig2,
                      kind2))
      .emit();
}

/// Given an enum variant's def, return the def of its enum and the associated fragment
fn handle_variant(cx: &DocContext, def: Def) -> Result<(Def, Option<String>), ()> {
    use rustc::ty::DefIdTree;

    let parent = if let Some(parent) = cx.tcx.parent(def.def_id()) {
        parent
    } else {
        return Err(())
    };
    let parent_def = Def::Enum(parent);
    let variant = cx.tcx.expect_variant_def(def);
    Ok((parent_def, Some(format!("{}.v", variant.name))))
}

const PRIMITIVES: &[(&str, Def)] = &[
    ("u8",    Def::PrimTy(hir::PrimTy::TyUint(syntax::ast::UintTy::U8))),
    ("u16",   Def::PrimTy(hir::PrimTy::TyUint(syntax::ast::UintTy::U16))),
    ("u32",   Def::PrimTy(hir::PrimTy::TyUint(syntax::ast::UintTy::U32))),
    ("u64",   Def::PrimTy(hir::PrimTy::TyUint(syntax::ast::UintTy::U64))),
    ("u128",  Def::PrimTy(hir::PrimTy::TyUint(syntax::ast::UintTy::U128))),
    ("usize", Def::PrimTy(hir::PrimTy::TyUint(syntax::ast::UintTy::Usize))),
    ("i8",    Def::PrimTy(hir::PrimTy::TyInt(syntax::ast::IntTy::I8))),
    ("i16",   Def::PrimTy(hir::PrimTy::TyInt(syntax::ast::IntTy::I16))),
    ("i32",   Def::PrimTy(hir::PrimTy::TyInt(syntax::ast::IntTy::I32))),
    ("i64",   Def::PrimTy(hir::PrimTy::TyInt(syntax::ast::IntTy::I64))),
    ("i128",  Def::PrimTy(hir::PrimTy::TyInt(syntax::ast::IntTy::I128))),
    ("isize", Def::PrimTy(hir::PrimTy::TyInt(syntax::ast::IntTy::Isize))),
    ("f32",   Def::PrimTy(hir::PrimTy::TyFloat(syntax::ast::FloatTy::F32))),
    ("f64",   Def::PrimTy(hir::PrimTy::TyFloat(syntax::ast::FloatTy::F64))),
    ("str",   Def::PrimTy(hir::PrimTy::TyStr)),
    ("bool",  Def::PrimTy(hir::PrimTy::TyBool)),
    ("char",  Def::PrimTy(hir::PrimTy::TyChar)),
];

fn is_primitive(path_str: &str, is_val: bool) -> Option<Def> {
    if is_val {
        None
    } else {
        PRIMITIVES.iter().find(|x| x.0 == path_str).map(|x| x.1)
    }
}

/// Resolve a given string as a path, along with whether or not it is
/// in the value namespace. Also returns an optional URL fragment in the case
/// of variants and methods
fn resolve(cx: &DocContext, path_str: &str, is_val: bool) -> Result<(Def, Option<String>), ()> {
    // In case we're in a module, try to resolve the relative
    // path
    if let Some(id) = cx.mod_ids.borrow().last() {
        let result = cx.resolver.borrow_mut()
                                .with_scope(*id,
            |resolver| {
                resolver.resolve_str_path_error(DUMMY_SP,
                                                &path_str, is_val)
        });

        if let Ok(result) = result {
            // In case this is a trait item, skip the
            // early return and try looking for the trait
            let value = match result.def {
                Def::Method(_) | Def::AssociatedConst(_) => true,
                Def::AssociatedTy(_) => false,
                Def::Variant(_) => return handle_variant(cx, result.def),
                // not a trait item, just return what we found
                _ => return Ok((result.def, None))
            };

            if value != is_val {
                return Err(())
            }
        } else if let Some(prim) = is_primitive(path_str, is_val) {
            return Ok((prim, Some(path_str.to_owned())))
        } else {
            // If resolution failed, it may still be a method
            // because methods are not handled by the resolver
            // If so, bail when we're not looking for a value
            if !is_val {
                return Err(())
            }
        }

        // Try looking for methods and associated items
        let mut split = path_str.rsplitn(2, "::");
        let mut item_name = if let Some(first) = split.next() {
            first
        } else {
            return Err(())
        };

        let mut path = if let Some(second) = split.next() {
            second
        } else {
            return Err(())
        };

        let ty = cx.resolver.borrow_mut()
                            .with_scope(*id,
            |resolver| {
                resolver.resolve_str_path_error(DUMMY_SP, &path, false)
        })?;
        match ty.def {
            Def::Struct(did) | Def::Union(did) | Def::Enum(did) | Def::TyAlias(did) => {
                let item = cx.tcx.inherent_impls(did).iter()
                                 .flat_map(|imp| cx.tcx.associated_items(*imp))
                                 .find(|item| item.name == item_name);
                if let Some(item) = item {
                    let out = match item.kind {
                        ty::AssociatedKind::Method if is_val => "method",
                        ty::AssociatedKind::Const if is_val => "associatedconstant",
                        _ => return Err(())
                    };
                    Ok((ty.def, Some(format!("{}.{}", out, item_name))))
                } else {
                    let is_enum = match ty.def {
                        Def::Enum(_) => true,
                        _ => false,
                    };
                    let elem = if is_enum {
                        cx.tcx.adt_def(did).all_fields().find(|item| item.ident.name == item_name)
                    } else {
                        cx.tcx.adt_def(did)
                              .non_enum_variant()
                              .fields
                              .iter()
                              .find(|item| item.ident.name == item_name)
                    };
                    if let Some(item) = elem {
                        Ok((ty.def,
                            Some(format!("{}.{}",
                                         if is_enum { "variant" } else { "structfield" },
                                         item.ident))))
                    } else {
                        Err(())
                    }
                }
            }
            Def::Trait(did) => {
                let item = cx.tcx.associated_item_def_ids(did).iter()
                             .map(|item| cx.tcx.associated_item(*item))
                             .find(|item| item.name == item_name);
                if let Some(item) = item {
                    let kind = match item.kind {
                        ty::AssociatedKind::Const if is_val => "associatedconstant",
                        ty::AssociatedKind::Type if !is_val => "associatedtype",
                        ty::AssociatedKind::Method if is_val => {
                            if item.defaultness.has_value() {
                                "method"
                            } else {
                                "tymethod"
                            }
                        }
                        _ => return Err(())
                    };

                    Ok((ty.def, Some(format!("{}.{}", kind, item_name))))
                } else {
                    Err(())
                }
            }
            _ => Err(())
        }
    } else {
        Err(())
    }
}

/// Resolve a string as a macro
fn macro_resolve(cx: &DocContext, path_str: &str) -> Option<Def> {
    use syntax::ext::base::{MacroKind, SyntaxExtension};
    use syntax::ext::hygiene::Mark;
    let segment = ast::PathSegment::from_ident(Ident::from_str(path_str));
    let path = ast::Path { segments: vec![segment], span: DUMMY_SP };
    let mut resolver = cx.resolver.borrow_mut();
    let mark = Mark::root();
    let res = resolver
        .resolve_macro_to_def_inner(mark, &path, MacroKind::Bang, false);
    if let Ok(def) = res {
        if let SyntaxExtension::DeclMacro(..) = *resolver.get_macro(def) {
            Some(def)
        } else {
            None
        }
    } else if let Some(def) = resolver.all_macros.get(&Symbol::intern(path_str)) {
        Some(*def)
    } else {
        None
    }
}

#[derive(Debug)]
enum PathKind {
    /// can be either value or type, not a macro
    Unknown,
    /// macro
    Macro,
    /// values, functions, consts, statics, everything in the value namespace
    Value,
    /// types, traits, everything in the type namespace
    Type,
}

fn resolution_failure(
    cx: &DocContext,
    attrs: &Attributes,
    path_str: &str,
    dox: &str,
    link_range: Option<Range<usize>>,
) {
    let sp = span_of_attrs(attrs);
    let msg = format!("`[{}]` cannot be resolved, ignoring it...", path_str);

    let code_dox = sp.to_src(cx);

    let doc_comment_padding = 3;
    let mut diag = if let Some(link_range) = link_range {
        // blah blah blah\nblah\nblah [blah] blah blah\nblah blah
        //                       ^    ~~~~~~
        //                       |    link_range
        //                       last_new_line_offset

        let mut diag;
        if dox.lines().count() == code_dox.lines().count() {
            let line_offset = dox[..link_range.start].lines().count();
            // The span starts in the `///`, so we don't have to account for the leading whitespace
            let code_dox_len = if line_offset <= 1 {
                doc_comment_padding
            } else {
                // The first `///`
                doc_comment_padding +
                    // Each subsequent leading whitespace and `///`
                    code_dox.lines().skip(1).take(line_offset - 1).fold(0, |sum, line| {
                        sum + doc_comment_padding + line.len() - line.trim().len()
                    })
            };

            // Extract the specific span
            let sp = sp.from_inner_byte_pos(
                link_range.start + code_dox_len,
                link_range.end + code_dox_len,
            );

            diag = cx.tcx.struct_span_lint_node(lint::builtin::INTRA_DOC_LINK_RESOLUTION_FAILURE,
                                                NodeId::new(0),
                                                sp,
                                                &msg);
            diag.span_label(sp, "cannot be resolved, ignoring");
        } else {
            diag = cx.tcx.struct_span_lint_node(lint::builtin::INTRA_DOC_LINK_RESOLUTION_FAILURE,
                                                NodeId::new(0),
                                                sp,
                                                &msg);

            let last_new_line_offset = dox[..link_range.start].rfind('\n').map_or(0, |n| n + 1);
            let line = dox[last_new_line_offset..].lines().next().unwrap_or("");

            // Print the line containing the `link_range` and manually mark it with '^'s
            diag.note(&format!(
                "the link appears in this line:\n\n{line}\n\
                 {indicator: <before$}{indicator:^<found$}",
                line=line,
                indicator="",
                before=link_range.start - last_new_line_offset,
                found=link_range.len(),
            ));
        }
        diag
    } else {
        cx.tcx.struct_span_lint_node(lint::builtin::INTRA_DOC_LINK_RESOLUTION_FAILURE,
                                     NodeId::new(0),
                                     sp,
                                     &msg)
    };
    diag.help("to escape `[` and `]` characters, just add '\\' before them like \
               `\\[` or `\\]`");
    diag.emit();
}

impl Clean<Attributes> for [ast::Attribute] {
    fn clean(&self, cx: &DocContext) -> Attributes {
        let mut attrs = Attributes::from_ast(cx.sess().diagnostic(), self);

        if UnstableFeatures::from_environment().is_nightly_build() {
            let dox = attrs.collapsed_doc_value().unwrap_or_else(String::new);
            for (ori_link, link_range) in markdown_links(&dox) {
                // bail early for real links
                if ori_link.contains('/') {
                    continue;
                }
                let link = ori_link.replace("`", "");
                let (def, fragment) = {
                    let mut kind = PathKind::Unknown;
                    let path_str = if let Some(prefix) =
                        ["struct@", "enum@", "type@",
                         "trait@", "union@"].iter()
                                          .find(|p| link.starts_with(**p)) {
                        kind = PathKind::Type;
                        link.trim_left_matches(prefix)
                    } else if let Some(prefix) =
                        ["const@", "static@",
                         "value@", "function@", "mod@",
                         "fn@", "module@", "method@"]
                            .iter().find(|p| link.starts_with(**p)) {
                        kind = PathKind::Value;
                        link.trim_left_matches(prefix)
                    } else if link.ends_with("()") {
                        kind = PathKind::Value;
                        link.trim_right_matches("()")
                    } else if link.starts_with("macro@") {
                        kind = PathKind::Macro;
                        link.trim_left_matches("macro@")
                    } else if link.ends_with('!') {
                        kind = PathKind::Macro;
                        link.trim_right_matches('!')
                    } else {
                        &link[..]
                    }.trim();

                    if path_str.contains(|ch: char| !(ch.is_alphanumeric() ||
                                                      ch == ':' || ch == '_')) {
                        continue;
                    }

                    match kind {
                        PathKind::Value => {
                            if let Ok(def) = resolve(cx, path_str, true) {
                                def
                            } else {
                                resolution_failure(cx, &attrs, path_str, &dox, link_range);
                                // this could just be a normal link or a broken link
                                // we could potentially check if something is
                                // "intra-doc-link-like" and warn in that case
                                continue;
                            }
                        }
                        PathKind::Type => {
                            if let Ok(def) = resolve(cx, path_str, false) {
                                def
                            } else {
                                resolution_failure(cx, &attrs, path_str, &dox, link_range);
                                // this could just be a normal link
                                continue;
                            }
                        }
                        PathKind::Unknown => {
                            // try everything!
                            if let Some(macro_def) = macro_resolve(cx, path_str) {
                                if let Ok(type_def) = resolve(cx, path_str, false) {
                                    let (type_kind, article, type_disambig)
                                        = type_ns_kind(type_def.0, path_str);
                                    ambiguity_error(cx, &attrs, path_str,
                                                    article, type_kind, &type_disambig,
                                                    "a", "macro", &format!("macro@{}", path_str));
                                    continue;
                                } else if let Ok(value_def) = resolve(cx, path_str, true) {
                                    let (value_kind, value_disambig)
                                        = value_ns_kind(value_def.0, path_str)
                                            .expect("struct and mod cases should have been \
                                                     caught in previous branch");
                                    ambiguity_error(cx, &attrs, path_str,
                                                    "a", value_kind, &value_disambig,
                                                    "a", "macro", &format!("macro@{}", path_str));
                                }
                                (macro_def, None)
                            } else if let Ok(type_def) = resolve(cx, path_str, false) {
                                // It is imperative we search for not-a-value first
                                // Otherwise we will find struct ctors for when we are looking
                                // for structs, and the link won't work.
                                // if there is something in both namespaces
                                if let Ok(value_def) = resolve(cx, path_str, true) {
                                    let kind = value_ns_kind(value_def.0, path_str);
                                    if let Some((value_kind, value_disambig)) = kind {
                                        let (type_kind, article, type_disambig)
                                            = type_ns_kind(type_def.0, path_str);
                                        ambiguity_error(cx, &attrs, path_str,
                                                        article, type_kind, &type_disambig,
                                                        "a", value_kind, &value_disambig);
                                        continue;
                                    }
                                }
                                type_def
                            } else if let Ok(value_def) = resolve(cx, path_str, true) {
                                value_def
                            } else {
                                resolution_failure(cx, &attrs, path_str, &dox, link_range);
                                // this could just be a normal link
                                continue;
                            }
                        }
                        PathKind::Macro => {
                            if let Some(def) = macro_resolve(cx, path_str) {
                                (def, None)
                            } else {
                                resolution_failure(cx, &attrs, path_str, &dox, link_range);
                                continue
                            }
                        }
                    }
                };

                if let Def::PrimTy(_) = def {
                    attrs.links.push((ori_link, None, fragment));
                } else {
                    let id = register_def(cx, def);
                    attrs.links.push((ori_link, Some(id), fragment));
                }
            }

            cx.sess().abort_if_errors();
        }

        attrs
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct TyParam {
    pub name: String,
    pub did: DefId,
    pub bounds: Vec<TyParamBound>,
    pub default: Option<Type>,
    pub synthetic: Option<hir::SyntheticTyParamKind>,
}

impl Clean<TyParam> for hir::TyParam {
    fn clean(&self, cx: &DocContext) -> TyParam {
        TyParam {
            name: self.name.clean(cx),
            did: cx.tcx.hir.local_def_id(self.id),
            bounds: self.bounds.clean(cx),
            default: self.default.clean(cx),
            synthetic: self.synthetic,
        }
    }
}

impl<'tcx> Clean<TyParam> for ty::GenericParamDef {
    fn clean(&self, cx: &DocContext) -> TyParam {
        cx.renderinfo.borrow_mut().external_typarams.insert(self.def_id, self.name.clean(cx));
        let has_default = match self.kind {
            ty::GenericParamDefKind::Type { has_default, .. } => has_default,
            _ => panic!("tried to convert a non-type GenericParamDef as a type")
        };
        TyParam {
            name: self.name.clean(cx),
            did: self.def_id,
            bounds: vec![], // these are filled in from the where-clauses
            default: if has_default {
                Some(cx.tcx.type_of(self.def_id).clean(cx))
            } else {
                None
            },
            synthetic: None,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum TyParamBound {
    RegionBound(Lifetime),
    TraitBound(PolyTrait, hir::TraitBoundModifier)
}

impl TyParamBound {
    fn maybe_sized(cx: &DocContext) -> TyParamBound {
        let did = cx.tcx.require_lang_item(lang_items::SizedTraitLangItem);
        let empty = cx.tcx.intern_substs(&[]);
        let path = external_path(cx, &cx.tcx.item_name(did).as_str(),
            Some(did), false, vec![], empty);
        inline::record_extern_fqn(cx, did, TypeKind::Trait);
        TraitBound(PolyTrait {
            trait_: ResolvedPath {
                path,
                typarams: None,
                did,
                is_generic: false,
            },
            generic_params: Vec::new(),
        }, hir::TraitBoundModifier::Maybe)
    }

    fn is_sized_bound(&self, cx: &DocContext) -> bool {
        use rustc::hir::TraitBoundModifier as TBM;
        if let TyParamBound::TraitBound(PolyTrait { ref trait_, .. }, TBM::None) = *self {
            if trait_.def_id() == cx.tcx.lang_items().sized_trait() {
                return true;
            }
        }
        false
    }

    fn get_poly_trait(&self) -> Option<PolyTrait> {
        if let TyParamBound::TraitBound(ref p, _) = *self {
            return Some(p.clone())
        }
        None
    }

    fn get_trait_type(&self) -> Option<Type> {

        if let TyParamBound::TraitBound(PolyTrait { ref trait_, .. }, _) = *self {
            return Some(trait_.clone());
        }
        None
    }
}

impl Clean<TyParamBound> for hir::TyParamBound {
    fn clean(&self, cx: &DocContext) -> TyParamBound {
        match *self {
            hir::RegionTyParamBound(lt) => RegionBound(lt.clean(cx)),
            hir::TraitTyParamBound(ref t, modifier) => TraitBound(t.clean(cx), modifier),
        }
    }
}

fn external_path_params(cx: &DocContext, trait_did: Option<DefId>, has_self: bool,
                        bindings: Vec<TypeBinding>, substs: &Substs) -> PathParameters {
    let lifetimes = substs.regions().filter_map(|v| v.clean(cx)).collect();
    let types = substs.types().skip(has_self as usize).collect::<Vec<_>>();

    match trait_did {
        // Attempt to sugar an external path like Fn<(A, B,), C> to Fn(A, B) -> C
        Some(did) if cx.tcx.lang_items().fn_trait_kind(did).is_some() => {
            assert_eq!(types.len(), 1);
            let inputs = match types[0].sty {
                ty::TyTuple(ref tys) => tys.iter().map(|t| t.clean(cx)).collect(),
                _ => {
                    return PathParameters::AngleBracketed {
                        lifetimes,
                        types: types.clean(cx),
                        bindings,
                    }
                }
            };
            let output = None;
            // FIXME(#20299) return type comes from a projection now
            // match types[1].sty {
            //     ty::TyTuple(ref v) if v.is_empty() => None, // -> ()
            //     _ => Some(types[1].clean(cx))
            // };
            PathParameters::Parenthesized {
                inputs,
                output,
            }
        },
        _ => {
            PathParameters::AngleBracketed {
                lifetimes,
                types: types.clean(cx),
                bindings,
            }
        }
    }
}

// trait_did should be set to a trait's DefId if called on a TraitRef, in order to sugar
// from Fn<(A, B,), C> to Fn(A, B) -> C
fn external_path(cx: &DocContext, name: &str, trait_did: Option<DefId>, has_self: bool,
                 bindings: Vec<TypeBinding>, substs: &Substs) -> Path {
    Path {
        global: false,
        def: Def::Err,
        segments: vec![PathSegment {
            name: name.to_string(),
            params: external_path_params(cx, trait_did, has_self, bindings, substs)
        }],
    }
}

impl<'a, 'tcx> Clean<TyParamBound> for (&'a ty::TraitRef<'tcx>, Vec<TypeBinding>) {
    fn clean(&self, cx: &DocContext) -> TyParamBound {
        let (trait_ref, ref bounds) = *self;
        inline::record_extern_fqn(cx, trait_ref.def_id, TypeKind::Trait);
        let path = external_path(cx, &cx.tcx.item_name(trait_ref.def_id).as_str(),
                                 Some(trait_ref.def_id), true, bounds.clone(), trait_ref.substs);

        debug!("ty::TraitRef\n  subst: {:?}\n", trait_ref.substs);

        // collect any late bound regions
        let mut late_bounds = vec![];
        for ty_s in trait_ref.input_types().skip(1) {
            if let ty::TyTuple(ts) = ty_s.sty {
                for &ty_s in ts {
                    if let ty::TyRef(ref reg, _, _) = ty_s.sty {
                        if let &ty::RegionKind::ReLateBound(..) = *reg {
                            debug!("  hit an ReLateBound {:?}", reg);
                            if let Some(lt) = reg.clean(cx) {
                                late_bounds.push(GenericParamDef::Lifetime(lt));
                            }
                        }
                    }
                }
            }
        }

        TraitBound(
            PolyTrait {
                trait_: ResolvedPath {
                    path,
                    typarams: None,
                    did: trait_ref.def_id,
                    is_generic: false,
                },
                generic_params: late_bounds,
            },
            hir::TraitBoundModifier::None
        )
    }
}

impl<'tcx> Clean<TyParamBound> for ty::TraitRef<'tcx> {
    fn clean(&self, cx: &DocContext) -> TyParamBound {
        (self, vec![]).clean(cx)
    }
}

impl<'tcx> Clean<Option<Vec<TyParamBound>>> for Substs<'tcx> {
    fn clean(&self, cx: &DocContext) -> Option<Vec<TyParamBound>> {
        let mut v = Vec::new();
        v.extend(self.regions().filter_map(|r| r.clean(cx))
                     .map(RegionBound));
        v.extend(self.types().map(|t| TraitBound(PolyTrait {
            trait_: t.clean(cx),
            generic_params: Vec::new(),
        }, hir::TraitBoundModifier::None)));
        if !v.is_empty() {Some(v)} else {None}
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct Lifetime(String);

impl Lifetime {
    pub fn get_ref<'a>(&'a self) -> &'a str {
        let Lifetime(ref s) = *self;
        let s: &'a str = s;
        s
    }

    pub fn statik() -> Lifetime {
        Lifetime("'static".to_string())
    }
}

impl Clean<Lifetime> for hir::Lifetime {
    fn clean(&self, cx: &DocContext) -> Lifetime {
        if self.id != ast::DUMMY_NODE_ID {
            let hir_id = cx.tcx.hir.node_to_hir_id(self.id);
            let def = cx.tcx.named_region(hir_id);
            match def {
                Some(rl::Region::EarlyBound(_, node_id, _)) |
                Some(rl::Region::LateBound(_, node_id, _)) |
                Some(rl::Region::Free(_, node_id)) => {
                    if let Some(lt) = cx.lt_substs.borrow().get(&node_id).cloned() {
                        return lt;
                    }
                }
                _ => {}
            }
        }
        Lifetime(self.name.name().to_string())
    }
}

impl Clean<Lifetime> for hir::LifetimeDef {
    fn clean(&self, _: &DocContext) -> Lifetime {
        if self.bounds.len() > 0 {
            let mut s = format!("{}: {}",
                                self.lifetime.name.name(),
                                self.bounds[0].name.name());
            for bound in self.bounds.iter().skip(1) {
                s.push_str(&format!(" + {}", bound.name.name()));
            }
            Lifetime(s)
        } else {
            Lifetime(self.lifetime.name.name().to_string())
        }
    }
}

impl<'tcx> Clean<Lifetime> for ty::GenericParamDef {
    fn clean(&self, _cx: &DocContext) -> Lifetime {
        Lifetime(self.name.to_string())
    }
}

impl Clean<Option<Lifetime>> for ty::RegionKind {
    fn clean(&self, cx: &DocContext) -> Option<Lifetime> {
        match *self {
            ty::ReStatic => Some(Lifetime::statik()),
            ty::ReLateBound(_, ty::BrNamed(_, name)) => Some(Lifetime(name.to_string())),
            ty::ReEarlyBound(ref data) => Some(Lifetime(data.name.clean(cx))),

            ty::ReLateBound(..) |
            ty::ReFree(..) |
            ty::ReScope(..) |
            ty::ReVar(..) |
            ty::ReSkolemized(..) |
            ty::ReEmpty |
            ty::ReClosureBound(_) |
            ty::ReCanonical(_) |
            ty::ReErased => None
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum WherePredicate {
    BoundPredicate { ty: Type, bounds: Vec<TyParamBound> },
    RegionPredicate { lifetime: Lifetime, bounds: Vec<Lifetime>},
    EqPredicate { lhs: Type, rhs: Type },
}

impl Clean<WherePredicate> for hir::WherePredicate {
    fn clean(&self, cx: &DocContext) -> WherePredicate {
        match *self {
            hir::WherePredicate::BoundPredicate(ref wbp) => {
                WherePredicate::BoundPredicate {
                    ty: wbp.bounded_ty.clean(cx),
                    bounds: wbp.bounds.clean(cx)
                }
            }

            hir::WherePredicate::RegionPredicate(ref wrp) => {
                WherePredicate::RegionPredicate {
                    lifetime: wrp.lifetime.clean(cx),
                    bounds: wrp.bounds.clean(cx)
                }
            }

            hir::WherePredicate::EqPredicate(ref wrp) => {
                WherePredicate::EqPredicate {
                    lhs: wrp.lhs_ty.clean(cx),
                    rhs: wrp.rhs_ty.clean(cx)
                }
            }
        }
    }
}

impl<'a> Clean<WherePredicate> for ty::Predicate<'a> {
    fn clean(&self, cx: &DocContext) -> WherePredicate {
        use rustc::ty::Predicate;

        match *self {
            Predicate::Trait(ref pred) => pred.clean(cx),
            Predicate::Subtype(ref pred) => pred.clean(cx),
            Predicate::RegionOutlives(ref pred) => pred.clean(cx),
            Predicate::TypeOutlives(ref pred) => pred.clean(cx),
            Predicate::Projection(ref pred) => pred.clean(cx),
            Predicate::WellFormed(_) => panic!("not user writable"),
            Predicate::ObjectSafe(_) => panic!("not user writable"),
            Predicate::ClosureKind(..) => panic!("not user writable"),
            Predicate::ConstEvaluatable(..) => panic!("not user writable"),
        }
    }
}

impl<'a> Clean<WherePredicate> for ty::TraitPredicate<'a> {
    fn clean(&self, cx: &DocContext) -> WherePredicate {
        WherePredicate::BoundPredicate {
            ty: self.trait_ref.self_ty().clean(cx),
            bounds: vec![self.trait_ref.clean(cx)]
        }
    }
}

impl<'tcx> Clean<WherePredicate> for ty::SubtypePredicate<'tcx> {
    fn clean(&self, _cx: &DocContext) -> WherePredicate {
        panic!("subtype predicates are an internal rustc artifact \
                and should not be seen by rustdoc")
    }
}

impl<'tcx> Clean<WherePredicate> for ty::OutlivesPredicate<ty::Region<'tcx>, ty::Region<'tcx>> {
    fn clean(&self, cx: &DocContext) -> WherePredicate {
        let ty::OutlivesPredicate(ref a, ref b) = *self;
        WherePredicate::RegionPredicate {
            lifetime: a.clean(cx).unwrap(),
            bounds: vec![b.clean(cx).unwrap()]
        }
    }
}

impl<'tcx> Clean<WherePredicate> for ty::OutlivesPredicate<Ty<'tcx>, ty::Region<'tcx>> {
    fn clean(&self, cx: &DocContext) -> WherePredicate {
        let ty::OutlivesPredicate(ref ty, ref lt) = *self;

        WherePredicate::BoundPredicate {
            ty: ty.clean(cx),
            bounds: vec![TyParamBound::RegionBound(lt.clean(cx).unwrap())]
        }
    }
}

impl<'tcx> Clean<WherePredicate> for ty::ProjectionPredicate<'tcx> {
    fn clean(&self, cx: &DocContext) -> WherePredicate {
        WherePredicate::EqPredicate {
            lhs: self.projection_ty.clean(cx),
            rhs: self.ty.clean(cx)
        }
    }
}

impl<'tcx> Clean<Type> for ty::ProjectionTy<'tcx> {
    fn clean(&self, cx: &DocContext) -> Type {
        let trait_ = match self.trait_ref(cx.tcx).clean(cx) {
            TyParamBound::TraitBound(t, _) => t.trait_,
            TyParamBound::RegionBound(_) => {
                panic!("cleaning a trait got a region")
            }
        };
        Type::QPath {
            name: cx.tcx.associated_item(self.item_def_id).name.clean(cx),
            self_type: box self.self_ty().clean(cx),
            trait_: box trait_
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum GenericParamDef {
    Lifetime(Lifetime),
    Type(TyParam),
}

impl GenericParamDef {
    pub fn is_synthetic_type_param(&self) -> bool {
        match self {
            GenericParamDef::Type(ty) => ty.synthetic.is_some(),
            GenericParamDef::Lifetime(_) => false,
        }
    }
}

impl Clean<GenericParamDef> for hir::GenericParam {
    fn clean(&self, cx: &DocContext) -> GenericParamDef {
        match *self {
            hir::GenericParam::Lifetime(ref l) => GenericParamDef::Lifetime(l.clean(cx)),
            hir::GenericParam::Type(ref t) => GenericParamDef::Type(t.clean(cx)),
        }
    }
}

// maybe use a Generic enum and use Vec<Generic>?
#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Default, Hash)]
pub struct Generics {
    pub params: Vec<GenericParamDef>,
    pub where_predicates: Vec<WherePredicate>,
}

impl Clean<Generics> for hir::Generics {
    fn clean(&self, cx: &DocContext) -> Generics {
        // Synthetic type-parameters are inserted after normal ones.
        // In order for normal parameters to be able to refer to synthetic ones,
        // scans them first.
        fn is_impl_trait(param: &hir::GenericParam) -> bool {
            if let hir::GenericParam::Type(ref tp) = param {
                tp.synthetic == Some(hir::SyntheticTyParamKind::ImplTrait)
            } else {
                false
            }
        }
        let impl_trait_params = self.params
            .iter()
            .filter(|p| is_impl_trait(p))
            .map(|p| {
                let p = p.clean(cx);
                if let GenericParamDef::Type(ref tp) = p {
                    cx.impl_trait_bounds
                        .borrow_mut()
                        .insert(tp.did, tp.bounds.clone());
                } else {
                    unreachable!()
                }
                p
            })
            .collect::<Vec<_>>();

        let mut params = Vec::with_capacity(self.params.len());
        for p in self.params.iter().filter(|p| !is_impl_trait(p)) {
            let p = p.clean(cx);
            params.push(p);
        }
        params.extend(impl_trait_params);

        let mut g = Generics {
            params,
            where_predicates: self.where_clause.predicates.clean(cx)
        };

        // Some duplicates are generated for ?Sized bounds between type params and where
        // predicates. The point in here is to move the bounds definitions from type params
        // to where predicates when such cases occur.
        for where_pred in &mut g.where_predicates {
            match *where_pred {
                WherePredicate::BoundPredicate { ty: Generic(ref name), ref mut bounds } => {
                    if bounds.is_empty() {
                        for param in &mut g.params {
                            if let GenericParamDef::Type(ref mut type_param) = *param {
                                if &type_param.name == name {
                                    mem::swap(bounds, &mut type_param.bounds);
                                    break
                                }
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
        g
    }
}

impl<'a, 'tcx> Clean<Generics> for (&'a ty::Generics,
                                    &'a ty::GenericPredicates<'tcx>) {
    fn clean(&self, cx: &DocContext) -> Generics {
        use self::WherePredicate as WP;

        let (gens, preds) = *self;

        // Bounds in the type_params and lifetimes fields are repeated in the
        // predicates field (see rustc_typeck::collect::ty_generics), so remove
        // them.
        let stripped_typarams = gens.params.iter().filter_map(|param| {
            if let ty::GenericParamDefKind::Type {..} = param.kind {
                if param.name == keywords::SelfType.name().as_str() {
                    assert_eq!(param.index, 0);
                    None
                } else {
                    Some(param.clean(cx))
                }
            } else {
                None
            }
        }).collect::<Vec<TyParam>>();

        let mut where_predicates = preds.predicates.to_vec().clean(cx);

        // Type parameters and have a Sized bound by default unless removed with
        // ?Sized. Scan through the predicates and mark any type parameter with
        // a Sized bound, removing the bounds as we find them.
        //
        // Note that associated types also have a sized bound by default, but we
        // don't actually know the set of associated types right here so that's
        // handled in cleaning associated types
        let mut sized_params = FxHashSet();
        where_predicates.retain(|pred| {
            match *pred {
                WP::BoundPredicate { ty: Generic(ref g), ref bounds } => {
                    if bounds.iter().any(|b| b.is_sized_bound(cx)) {
                        sized_params.insert(g.clone());
                        false
                    } else {
                        true
                    }
                }
                _ => true,
            }
        });

        // Run through the type parameters again and insert a ?Sized
        // unbound for any we didn't find to be Sized.
        for tp in &stripped_typarams {
            if !sized_params.contains(&tp.name) {
                where_predicates.push(WP::BoundPredicate {
                    ty: Type::Generic(tp.name.clone()),
                    bounds: vec![TyParamBound::maybe_sized(cx)],
                })
            }
        }

        // It would be nice to collect all of the bounds on a type and recombine
        // them if possible, to avoid e.g. `where T: Foo, T: Bar, T: Sized, T: 'a`
        // and instead see `where T: Foo + Bar + Sized + 'a`

        Generics {
            params: gens.params
                        .iter()
                        .flat_map(|param| {
                            if let ty::GenericParamDefKind::Lifetime = param.kind {
                                Some(GenericParamDef::Lifetime(param.clean(cx)))
                            } else {
                                None
                            }
                        }).chain(
                            simplify::ty_params(stripped_typarams)
                                .into_iter()
                                .map(|tp| GenericParamDef::Type(tp))
                        )
                        .collect(),
            where_predicates: simplify::where_clauses(cx, where_predicates),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Method {
    pub generics: Generics,
    pub unsafety: hir::Unsafety,
    pub constness: hir::Constness,
    pub decl: FnDecl,
    pub abi: Abi,
}

impl<'a> Clean<Method> for (&'a hir::MethodSig, &'a hir::Generics, hir::BodyId) {
    fn clean(&self, cx: &DocContext) -> Method {
        let (generics, decl) = enter_impl_trait(cx, || {
            (self.1.clean(cx), (&*self.0.decl, self.2).clean(cx))
        });
        Method {
            decl,
            generics,
            unsafety: self.0.unsafety,
            constness: self.0.constness,
            abi: self.0.abi
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct TyMethod {
    pub unsafety: hir::Unsafety,
    pub decl: FnDecl,
    pub generics: Generics,
    pub abi: Abi,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Function {
    pub decl: FnDecl,
    pub generics: Generics,
    pub unsafety: hir::Unsafety,
    pub constness: hir::Constness,
    pub abi: Abi,
}

impl Clean<Item> for doctree::Function {
    fn clean(&self, cx: &DocContext) -> Item {
        let (generics, decl) = enter_impl_trait(cx, || {
            (self.generics.clean(cx), (&self.decl, self.body).clean(cx))
        });
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            inner: FunctionItem(Function {
                decl,
                generics,
                unsafety: self.unsafety,
                constness: self.constness,
                abi: self.abi,
            }),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct FnDecl {
    pub inputs: Arguments,
    pub output: FunctionRetTy,
    pub variadic: bool,
    pub attrs: Attributes,
}

impl FnDecl {
    pub fn has_self(&self) -> bool {
        self.inputs.values.len() > 0 && self.inputs.values[0].name == "self"
    }

    pub fn self_type(&self) -> Option<SelfTy> {
        self.inputs.values.get(0).and_then(|v| v.to_self())
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct Arguments {
    pub values: Vec<Argument>,
}

impl<'a> Clean<Arguments> for (&'a [P<hir::Ty>], &'a [Spanned<ast::Name>]) {
    fn clean(&self, cx: &DocContext) -> Arguments {
        Arguments {
            values: self.0.iter().enumerate().map(|(i, ty)| {
                let mut name = self.1.get(i).map(|n| n.node.to_string())
                                            .unwrap_or(String::new());
                if name.is_empty() {
                    name = "_".to_string();
                }
                Argument {
                    name,
                    type_: ty.clean(cx),
                }
            }).collect()
        }
    }
}

impl<'a> Clean<Arguments> for (&'a [P<hir::Ty>], hir::BodyId) {
    fn clean(&self, cx: &DocContext) -> Arguments {
        let body = cx.tcx.hir.body(self.1);

        Arguments {
            values: self.0.iter().enumerate().map(|(i, ty)| {
                Argument {
                    name: name_from_pat(&body.arguments[i].pat),
                    type_: ty.clean(cx),
                }
            }).collect()
        }
    }
}

impl<'a, A: Copy> Clean<FnDecl> for (&'a hir::FnDecl, A)
    where (&'a [P<hir::Ty>], A): Clean<Arguments>
{
    fn clean(&self, cx: &DocContext) -> FnDecl {
        FnDecl {
            inputs: (&self.0.inputs[..], self.1).clean(cx),
            output: self.0.output.clean(cx),
            variadic: self.0.variadic,
            attrs: Attributes::default()
        }
    }
}

impl<'a, 'tcx> Clean<FnDecl> for (DefId, ty::PolyFnSig<'tcx>) {
    fn clean(&self, cx: &DocContext) -> FnDecl {
        let (did, sig) = *self;
        let mut names = if cx.tcx.hir.as_local_node_id(did).is_some() {
            vec![].into_iter()
        } else {
            cx.tcx.fn_arg_names(did).into_iter()
        };

        FnDecl {
            output: Return(sig.skip_binder().output().clean(cx)),
            attrs: Attributes::default(),
            variadic: sig.skip_binder().variadic,
            inputs: Arguments {
                values: sig.skip_binder().inputs().iter().map(|t| {
                    Argument {
                        type_: t.clean(cx),
                        name: names.next().map_or("".to_string(), |name| name.to_string()),
                    }
                }).collect(),
            },
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct Argument {
    pub type_: Type,
    pub name: String,
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Debug)]
pub enum SelfTy {
    SelfValue,
    SelfBorrowed(Option<Lifetime>, Mutability),
    SelfExplicit(Type),
}

impl Argument {
    pub fn to_self(&self) -> Option<SelfTy> {
        if self.name != "self" {
            return None;
        }
        if self.type_.is_self_type() {
            return Some(SelfValue);
        }
        match self.type_ {
            BorrowedRef{ref lifetime, mutability, ref type_} if type_.is_self_type() => {
                Some(SelfBorrowed(lifetime.clone(), mutability))
            }
            _ => Some(SelfExplicit(self.type_.clone()))
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum FunctionRetTy {
    Return(Type),
    DefaultReturn,
}

impl Clean<FunctionRetTy> for hir::FunctionRetTy {
    fn clean(&self, cx: &DocContext) -> FunctionRetTy {
        match *self {
            hir::Return(ref typ) => Return(typ.clean(cx)),
            hir::DefaultReturn(..) => DefaultReturn,
        }
    }
}

impl GetDefId for FunctionRetTy {
    fn def_id(&self) -> Option<DefId> {
        match *self {
            Return(ref ty) => ty.def_id(),
            DefaultReturn => None,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Trait {
    pub auto: bool,
    pub unsafety: hir::Unsafety,
    pub items: Vec<Item>,
    pub generics: Generics,
    pub bounds: Vec<TyParamBound>,
    pub is_spotlight: bool,
    pub is_auto: bool,
}

impl Clean<Item> for doctree::Trait {
    fn clean(&self, cx: &DocContext) -> Item {
        let attrs = self.attrs.clean(cx);
        let is_spotlight = attrs.has_doc_flag("spotlight");
        Item {
            name: Some(self.name.clean(cx)),
            attrs: attrs,
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: TraitItem(Trait {
                auto: self.is_auto.clean(cx),
                unsafety: self.unsafety,
                items: self.items.clean(cx),
                generics: self.generics.clean(cx),
                bounds: self.bounds.clean(cx),
                is_spotlight: is_spotlight,
                is_auto: self.is_auto.clean(cx),
            }),
        }
    }
}

impl Clean<bool> for hir::IsAuto {
    fn clean(&self, _: &DocContext) -> bool {
        match *self {
            hir::IsAuto::Yes => true,
            hir::IsAuto::No => false,
        }
    }
}

impl Clean<Type> for hir::TraitRef {
    fn clean(&self, cx: &DocContext) -> Type {
        resolve_type(cx, self.path.clean(cx), self.ref_id)
    }
}

impl Clean<PolyTrait> for hir::PolyTraitRef {
    fn clean(&self, cx: &DocContext) -> PolyTrait {
        PolyTrait {
            trait_: self.trait_ref.clean(cx),
            generic_params: self.bound_generic_params.clean(cx)
        }
    }
}

impl Clean<Item> for hir::TraitItem {
    fn clean(&self, cx: &DocContext) -> Item {
        let inner = match self.node {
            hir::TraitItemKind::Const(ref ty, default) => {
                AssociatedConstItem(ty.clean(cx),
                                    default.map(|e| print_const_expr(cx, e)))
            }
            hir::TraitItemKind::Method(ref sig, hir::TraitMethod::Provided(body)) => {
                MethodItem((sig, &self.generics, body).clean(cx))
            }
            hir::TraitItemKind::Method(ref sig, hir::TraitMethod::Required(ref names)) => {
                let (generics, decl) = enter_impl_trait(cx, || {
                    (self.generics.clean(cx), (&*sig.decl, &names[..]).clean(cx))
                });
                TyMethodItem(TyMethod {
                    unsafety: sig.unsafety.clone(),
                    decl,
                    generics,
                    abi: sig.abi
                })
            }
            hir::TraitItemKind::Type(ref bounds, ref default) => {
                AssociatedTypeItem(bounds.clean(cx), default.clean(cx))
            }
        };
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.span.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: None,
            stability: get_stability(cx, cx.tcx.hir.local_def_id(self.id)),
            deprecation: get_deprecation(cx, cx.tcx.hir.local_def_id(self.id)),
            inner,
        }
    }
}

impl Clean<Item> for hir::ImplItem {
    fn clean(&self, cx: &DocContext) -> Item {
        let inner = match self.node {
            hir::ImplItemKind::Const(ref ty, expr) => {
                AssociatedConstItem(ty.clean(cx),
                                    Some(print_const_expr(cx, expr)))
            }
            hir::ImplItemKind::Method(ref sig, body) => {
                MethodItem((sig, &self.generics, body).clean(cx))
            }
            hir::ImplItemKind::Type(ref ty) => TypedefItem(Typedef {
                type_: ty.clean(cx),
                generics: Generics::default(),
            }, true),
        };
        Item {
            name: Some(self.name.clean(cx)),
            source: self.span.clean(cx),
            attrs: self.attrs.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: get_stability(cx, cx.tcx.hir.local_def_id(self.id)),
            deprecation: get_deprecation(cx, cx.tcx.hir.local_def_id(self.id)),
            inner,
        }
    }
}

impl<'tcx> Clean<Item> for ty::AssociatedItem {
    fn clean(&self, cx: &DocContext) -> Item {
        let inner = match self.kind {
            ty::AssociatedKind::Const => {
                let ty = cx.tcx.type_of(self.def_id);
                let default = if self.defaultness.has_value() {
                    Some(inline::print_inlined_const(cx, self.def_id))
                } else {
                    None
                };
                AssociatedConstItem(ty.clean(cx), default)
            }
            ty::AssociatedKind::Method => {
                let generics = (cx.tcx.generics_of(self.def_id),
                                &cx.tcx.predicates_of(self.def_id)).clean(cx);
                let sig = cx.tcx.fn_sig(self.def_id);
                let mut decl = (self.def_id, sig).clean(cx);

                if self.method_has_self_argument {
                    let self_ty = match self.container {
                        ty::ImplContainer(def_id) => {
                            cx.tcx.type_of(def_id)
                        }
                        ty::TraitContainer(_) => cx.tcx.mk_self_type()
                    };
                    let self_arg_ty = *sig.input(0).skip_binder();
                    if self_arg_ty == self_ty {
                        decl.inputs.values[0].type_ = Generic(String::from("Self"));
                    } else if let ty::TyRef(_, ty, _) = self_arg_ty.sty {
                        if ty == self_ty {
                            match decl.inputs.values[0].type_ {
                                BorrowedRef{ref mut type_, ..} => {
                                    **type_ = Generic(String::from("Self"))
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                let provided = match self.container {
                    ty::ImplContainer(_) => true,
                    ty::TraitContainer(_) => self.defaultness.has_value()
                };
                if provided {
                    let constness = if cx.tcx.is_const_fn(self.def_id) {
                        hir::Constness::Const
                    } else {
                        hir::Constness::NotConst
                    };
                    MethodItem(Method {
                        unsafety: sig.unsafety(),
                        generics,
                        decl,
                        abi: sig.abi(),
                        constness,
                    })
                } else {
                    TyMethodItem(TyMethod {
                        unsafety: sig.unsafety(),
                        generics,
                        decl,
                        abi: sig.abi(),
                    })
                }
            }
            ty::AssociatedKind::Type => {
                let my_name = self.name.clean(cx);

                if let ty::TraitContainer(did) = self.container {
                    // When loading a cross-crate associated type, the bounds for this type
                    // are actually located on the trait/impl itself, so we need to load
                    // all of the generics from there and then look for bounds that are
                    // applied to this associated type in question.
                    let predicates = cx.tcx.predicates_of(did);
                    let generics = (cx.tcx.generics_of(did), &predicates).clean(cx);
                    let mut bounds = generics.where_predicates.iter().filter_map(|pred| {
                        let (name, self_type, trait_, bounds) = match *pred {
                            WherePredicate::BoundPredicate {
                                ty: QPath { ref name, ref self_type, ref trait_ },
                                ref bounds
                            } => (name, self_type, trait_, bounds),
                            _ => return None,
                        };
                        if *name != my_name { return None }
                        match **trait_ {
                            ResolvedPath { did, .. } if did == self.container.id() => {}
                            _ => return None,
                        }
                        match **self_type {
                            Generic(ref s) if *s == "Self" => {}
                            _ => return None,
                        }
                        Some(bounds)
                    }).flat_map(|i| i.iter().cloned()).collect::<Vec<_>>();
                    // Our Sized/?Sized bound didn't get handled when creating the generics
                    // because we didn't actually get our whole set of bounds until just now
                    // (some of them may have come from the trait). If we do have a sized
                    // bound, we remove it, and if we don't then we add the `?Sized` bound
                    // at the end.
                    match bounds.iter().position(|b| b.is_sized_bound(cx)) {
                        Some(i) => { bounds.remove(i); }
                        None => bounds.push(TyParamBound::maybe_sized(cx)),
                    }

                    let ty = if self.defaultness.has_value() {
                        Some(cx.tcx.type_of(self.def_id))
                    } else {
                        None
                    };

                    AssociatedTypeItem(bounds, ty.clean(cx))
                } else {
                    TypedefItem(Typedef {
                        type_: cx.tcx.type_of(self.def_id).clean(cx),
                        generics: Generics {
                            params: Vec::new(),
                            where_predicates: Vec::new(),
                        },
                    }, true)
                }
            }
        };

        let visibility = match self.container {
            ty::ImplContainer(_) => self.vis.clean(cx),
            ty::TraitContainer(_) => None,
        };

        Item {
            name: Some(self.name.clean(cx)),
            visibility,
            stability: get_stability(cx, self.def_id),
            deprecation: get_deprecation(cx, self.def_id),
            def_id: self.def_id,
            attrs: inline::load_attrs(cx, self.def_id),
            source: cx.tcx.def_span(self.def_id).clean(cx),
            inner,
        }
    }
}

/// A trait reference, which may have higher ranked lifetimes.
#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct PolyTrait {
    pub trait_: Type,
    pub generic_params: Vec<GenericParamDef>,
}

/// A representation of a Type suitable for hyperlinking purposes. Ideally one can get the original
/// type out of the AST/TyCtxt given one of these, if more information is needed. Most importantly
/// it does not preserve mutability or boxes.
#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum Type {
    /// structs/enums/traits (most that'd be an hir::TyPath)
    ResolvedPath {
        path: Path,
        typarams: Option<Vec<TyParamBound>>,
        did: DefId,
        /// true if is a `T::Name` path for associated types
        is_generic: bool,
    },
    /// For parameterized types, so the consumer of the JSON don't go
    /// looking for types which don't exist anywhere.
    Generic(String),
    /// Primitives are the fixed-size numeric types (plus int/usize/float), char,
    /// arrays, slices, and tuples.
    Primitive(PrimitiveType),
    /// extern "ABI" fn
    BareFunction(Box<BareFunctionDecl>),
    Tuple(Vec<Type>),
    Slice(Box<Type>),
    Array(Box<Type>, String),
    Never,
    Unique(Box<Type>),
    RawPointer(Mutability, Box<Type>),
    BorrowedRef {
        lifetime: Option<Lifetime>,
        mutability: Mutability,
        type_: Box<Type>,
    },

    // <Type as Trait>::Name
    QPath {
        name: String,
        self_type: Box<Type>,
        trait_: Box<Type>
    },

    // _
    Infer,

    // impl TraitA+TraitB
    ImplTrait(Vec<TyParamBound>),
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Hash, Copy, Debug)]
pub enum PrimitiveType {
    Isize, I8, I16, I32, I64, I128,
    Usize, U8, U16, U32, U64, U128,
    F32, F64,
    Char,
    Bool,
    Str,
    Slice,
    Array,
    Tuple,
    Unit,
    RawPointer,
    Reference,
    Fn,
    Never,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Copy, Debug)]
pub enum TypeKind {
    Enum,
    Function,
    Module,
    Const,
    Static,
    Struct,
    Union,
    Trait,
    Variant,
    Typedef,
    Foreign,
    Macro,
}

pub trait GetDefId {
    fn def_id(&self) -> Option<DefId>;
}

impl<T: GetDefId> GetDefId for Option<T> {
    fn def_id(&self) -> Option<DefId> {
        self.as_ref().and_then(|d| d.def_id())
    }
}

impl Type {
    pub fn primitive_type(&self) -> Option<PrimitiveType> {
        match *self {
            Primitive(p) | BorrowedRef { type_: box Primitive(p), ..} => Some(p),
            Slice(..) | BorrowedRef { type_: box Slice(..), .. } => Some(PrimitiveType::Slice),
            Array(..) | BorrowedRef { type_: box Array(..), .. } => Some(PrimitiveType::Array),
            Tuple(ref tys) => if tys.is_empty() {
                Some(PrimitiveType::Unit)
            } else {
                Some(PrimitiveType::Tuple)
            },
            RawPointer(..) => Some(PrimitiveType::RawPointer),
            BorrowedRef { type_: box Generic(..), .. } => Some(PrimitiveType::Reference),
            BareFunction(..) => Some(PrimitiveType::Fn),
            Never => Some(PrimitiveType::Never),
            _ => None,
        }
    }

    pub fn is_generic(&self) -> bool {
        match *self {
            ResolvedPath { is_generic, .. } => is_generic,
            _ => false,
        }
    }

    pub fn is_self_type(&self) -> bool {
        match *self {
            Generic(ref name) => name == "Self",
            _ => false
        }
    }

    pub fn generics(&self) -> Option<&[Type]> {
        match *self {
            ResolvedPath { ref path, .. } => {
                path.segments.last().and_then(|seg| {
                    if let PathParameters::AngleBracketed { ref types, .. } = seg.params {
                        Some(&**types)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
}

impl GetDefId for Type {
    fn def_id(&self) -> Option<DefId> {
        match *self {
            ResolvedPath { did, .. } => Some(did),
            Primitive(p) => ::html::render::cache().primitive_locations.get(&p).cloned(),
            BorrowedRef { type_: box Generic(..), .. } =>
                Primitive(PrimitiveType::Reference).def_id(),
            BorrowedRef { ref type_, .. } => type_.def_id(),
            Tuple(ref tys) => if tys.is_empty() {
                Primitive(PrimitiveType::Unit).def_id()
            } else {
                Primitive(PrimitiveType::Tuple).def_id()
            },
            BareFunction(..) => Primitive(PrimitiveType::Fn).def_id(),
            Never => Primitive(PrimitiveType::Never).def_id(),
            Slice(..) => Primitive(PrimitiveType::Slice).def_id(),
            Array(..) => Primitive(PrimitiveType::Array).def_id(),
            RawPointer(..) => Primitive(PrimitiveType::RawPointer).def_id(),
            QPath { ref self_type, .. } => self_type.def_id(),
            _ => None,
        }
    }
}

impl PrimitiveType {
    fn from_str(s: &str) -> Option<PrimitiveType> {
        match s {
            "isize" => Some(PrimitiveType::Isize),
            "i8" => Some(PrimitiveType::I8),
            "i16" => Some(PrimitiveType::I16),
            "i32" => Some(PrimitiveType::I32),
            "i64" => Some(PrimitiveType::I64),
            "i128" => Some(PrimitiveType::I128),
            "usize" => Some(PrimitiveType::Usize),
            "u8" => Some(PrimitiveType::U8),
            "u16" => Some(PrimitiveType::U16),
            "u32" => Some(PrimitiveType::U32),
            "u64" => Some(PrimitiveType::U64),
            "u128" => Some(PrimitiveType::U128),
            "bool" => Some(PrimitiveType::Bool),
            "char" => Some(PrimitiveType::Char),
            "str" => Some(PrimitiveType::Str),
            "f32" => Some(PrimitiveType::F32),
            "f64" => Some(PrimitiveType::F64),
            "array" => Some(PrimitiveType::Array),
            "slice" => Some(PrimitiveType::Slice),
            "tuple" => Some(PrimitiveType::Tuple),
            "unit" => Some(PrimitiveType::Unit),
            "pointer" => Some(PrimitiveType::RawPointer),
            "reference" => Some(PrimitiveType::Reference),
            "fn" => Some(PrimitiveType::Fn),
            "never" => Some(PrimitiveType::Never),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        use self::PrimitiveType::*;
        match *self {
            Isize => "isize",
            I8 => "i8",
            I16 => "i16",
            I32 => "i32",
            I64 => "i64",
            I128 => "i128",
            Usize => "usize",
            U8 => "u8",
            U16 => "u16",
            U32 => "u32",
            U64 => "u64",
            U128 => "u128",
            F32 => "f32",
            F64 => "f64",
            Str => "str",
            Bool => "bool",
            Char => "char",
            Array => "array",
            Slice => "slice",
            Tuple => "tuple",
            Unit => "unit",
            RawPointer => "pointer",
            Reference => "reference",
            Fn => "fn",
            Never => "never",
        }
    }

    pub fn to_url_str(&self) -> &'static str {
        self.as_str()
    }
}

impl From<ast::IntTy> for PrimitiveType {
    fn from(int_ty: ast::IntTy) -> PrimitiveType {
        match int_ty {
            ast::IntTy::Isize => PrimitiveType::Isize,
            ast::IntTy::I8 => PrimitiveType::I8,
            ast::IntTy::I16 => PrimitiveType::I16,
            ast::IntTy::I32 => PrimitiveType::I32,
            ast::IntTy::I64 => PrimitiveType::I64,
            ast::IntTy::I128 => PrimitiveType::I128,
        }
    }
}

impl From<ast::UintTy> for PrimitiveType {
    fn from(uint_ty: ast::UintTy) -> PrimitiveType {
        match uint_ty {
            ast::UintTy::Usize => PrimitiveType::Usize,
            ast::UintTy::U8 => PrimitiveType::U8,
            ast::UintTy::U16 => PrimitiveType::U16,
            ast::UintTy::U32 => PrimitiveType::U32,
            ast::UintTy::U64 => PrimitiveType::U64,
            ast::UintTy::U128 => PrimitiveType::U128,
        }
    }
}

impl From<ast::FloatTy> for PrimitiveType {
    fn from(float_ty: ast::FloatTy) -> PrimitiveType {
        match float_ty {
            ast::FloatTy::F32 => PrimitiveType::F32,
            ast::FloatTy::F64 => PrimitiveType::F64,
        }
    }
}

impl Clean<Type> for hir::Ty {
    fn clean(&self, cx: &DocContext) -> Type {
        use rustc::hir::*;
        match self.node {
            TyNever => Never,
            TyPtr(ref m) => RawPointer(m.mutbl.clean(cx), box m.ty.clean(cx)),
            TyRptr(ref l, ref m) => {
                let lifetime = if l.is_elided() {
                    None
                } else {
                    Some(l.clean(cx))
                };
                BorrowedRef {lifetime: lifetime, mutability: m.mutbl.clean(cx),
                             type_: box m.ty.clean(cx)}
            }
            TySlice(ref ty) => Slice(box ty.clean(cx)),
            TyArray(ref ty, ref length) => {
                let def_id = cx.tcx.hir.local_def_id(length.id);
                let param_env = cx.tcx.param_env(def_id);
                let substs = Substs::identity_for_item(cx.tcx, def_id);
                let cid = GlobalId {
                    instance: ty::Instance::new(def_id, substs),
                    promoted: None
                };
                let length = cx.tcx.const_eval(param_env.and(cid)).unwrap_or_else(|_| {
                    ty::Const::unevaluated(cx.tcx, def_id, substs, cx.tcx.types.usize)
                });
                let length = print_const(cx, length);
                Array(box ty.clean(cx), length)
            },
            TyTup(ref tys) => Tuple(tys.clean(cx)),
            TyPath(hir::QPath::Resolved(None, ref path)) => {
                if let Some(new_ty) = cx.ty_substs.borrow().get(&path.def).cloned() {
                    return new_ty;
                }

                if let Def::TyParam(did) = path.def {
                    if let Some(bounds) = cx.impl_trait_bounds.borrow_mut().remove(&did) {
                        return ImplTrait(bounds);
                    }
                }

                let mut alias = None;
                if let Def::TyAlias(def_id) = path.def {
                    // Substitute private type aliases
                    if let Some(node_id) = cx.tcx.hir.as_local_node_id(def_id) {
                        if !cx.access_levels.borrow().is_exported(def_id) {
                            alias = Some(&cx.tcx.hir.expect_item(node_id).node);
                        }
                    }
                };

                if let Some(&hir::ItemTy(ref ty, ref generics)) = alias {
                    let provided_params = &path.segments.last().unwrap();
                    let mut ty_substs = FxHashMap();
                    let mut lt_substs = FxHashMap();
                    provided_params.with_parameters(|provided_params| {
                        let mut indices = GenericParamCount {
                            lifetimes: 0,
                            types: 0
                        };
                        for param in generics.params.iter() {
                            match param {
                                hir::GenericParam::Lifetime(lt_param) => {
                                    if let Some(lt) = provided_params.lifetimes
                                        .get(indices.lifetimes).cloned() {
                                        if !lt.is_elided() {
                                            let lt_def_id =
                                                cx.tcx.hir.local_def_id(lt_param.lifetime.id);
                                            lt_substs.insert(lt_def_id, lt.clean(cx));
                                        }
                                    }
                                    indices.lifetimes += 1;
                                }
                                hir::GenericParam::Type(ty_param) => {
                                    let ty_param_def =
                                        Def::TyParam(cx.tcx.hir.local_def_id(ty_param.id));
                                    if let Some(ty) = provided_params.types
                                        .get(indices.types).cloned() {
                                        ty_substs.insert(ty_param_def, ty.into_inner().clean(cx));
                                    } else if let Some(default) = ty_param.default.clone() {
                                        ty_substs.insert(ty_param_def,
                                                         default.into_inner().clean(cx));
                                    }
                                    indices.types += 1;
                                }
                            }
                        }
                    });
                    return cx.enter_alias(ty_substs, lt_substs, || ty.clean(cx));
                }
                resolve_type(cx, path.clean(cx), self.id)
            }
            TyPath(hir::QPath::Resolved(Some(ref qself), ref p)) => {
                let mut segments: Vec<_> = p.segments.clone().into();
                segments.pop();
                let trait_path = hir::Path {
                    span: p.span,
                    def: Def::Trait(cx.tcx.associated_item(p.def.def_id()).container.id()),
                    segments: segments.into(),
                };
                Type::QPath {
                    name: p.segments.last().unwrap().name.clean(cx),
                    self_type: box qself.clean(cx),
                    trait_: box resolve_type(cx, trait_path.clean(cx), self.id)
                }
            }
            TyPath(hir::QPath::TypeRelative(ref qself, ref segment)) => {
                let mut def = Def::Err;
                let ty = hir_ty_to_ty(cx.tcx, self);
                if let ty::TyProjection(proj) = ty.sty {
                    def = Def::Trait(proj.trait_ref(cx.tcx).def_id);
                }
                let trait_path = hir::Path {
                    span: self.span,
                    def,
                    segments: vec![].into(),
                };
                Type::QPath {
                    name: segment.name.clean(cx),
                    self_type: box qself.clean(cx),
                    trait_: box resolve_type(cx, trait_path.clean(cx), self.id)
                }
            }
            TyTraitObject(ref bounds, ref lifetime) => {
                match bounds[0].clean(cx).trait_ {
                    ResolvedPath { path, typarams: None, did, is_generic } => {
                        let mut bounds: Vec<_> = bounds[1..].iter().map(|bound| {
                            TraitBound(bound.clean(cx), hir::TraitBoundModifier::None)
                        }).collect();
                        if !lifetime.is_elided() {
                            bounds.push(RegionBound(lifetime.clean(cx)));
                        }
                        ResolvedPath {
                            path,
                            typarams: Some(bounds),
                            did,
                            is_generic,
                        }
                    }
                    _ => Infer // shouldn't happen
                }
            }
            TyBareFn(ref barefn) => BareFunction(box barefn.clean(cx)),
            TyImplTraitExistential(hir_id, _, _) => {
                match cx.tcx.hir.expect_item(hir_id.id).node {
                    hir::ItemExistential(ref exist_ty) => {
                        ImplTrait(exist_ty.bounds.clean(cx))
                    },
                    ref other => panic!("impl Trait pointed to {:#?}", other),
                }
            },
            TyInfer | TyErr => Infer,
            TyTypeof(..) => panic!("Unimplemented type {:?}", self.node),
        }
    }
}

impl<'tcx> Clean<Type> for Ty<'tcx> {
    fn clean(&self, cx: &DocContext) -> Type {
        match self.sty {
            ty::TyNever => Never,
            ty::TyBool => Primitive(PrimitiveType::Bool),
            ty::TyChar => Primitive(PrimitiveType::Char),
            ty::TyInt(int_ty) => Primitive(int_ty.into()),
            ty::TyUint(uint_ty) => Primitive(uint_ty.into()),
            ty::TyFloat(float_ty) => Primitive(float_ty.into()),
            ty::TyStr => Primitive(PrimitiveType::Str),
            ty::TySlice(ty) => Slice(box ty.clean(cx)),
            ty::TyArray(ty, n) => {
                let mut n = cx.tcx.lift(&n).unwrap();
                if let ConstVal::Unevaluated(def_id, substs) = n.val {
                    let param_env = cx.tcx.param_env(def_id);
                    let cid = GlobalId {
                        instance: ty::Instance::new(def_id, substs),
                        promoted: None
                    };
                    if let Ok(new_n) = cx.tcx.const_eval(param_env.and(cid)) {
                        n = new_n;
                    }
                };
                let n = print_const(cx, n);
                Array(box ty.clean(cx), n)
            }
            ty::TyRawPtr(mt) => RawPointer(mt.mutbl.clean(cx), box mt.ty.clean(cx)),
            ty::TyRef(r, ty, mutbl) => BorrowedRef {
                lifetime: r.clean(cx),
                mutability: mutbl.clean(cx),
                type_: box ty.clean(cx),
            },
            ty::TyFnDef(..) |
            ty::TyFnPtr(_) => {
                let ty = cx.tcx.lift(self).unwrap();
                let sig = ty.fn_sig(cx.tcx);
                BareFunction(box BareFunctionDecl {
                    unsafety: sig.unsafety(),
                    generic_params: Vec::new(),
                    decl: (cx.tcx.hir.local_def_id(ast::CRATE_NODE_ID), sig).clean(cx),
                    abi: sig.abi(),
                })
            }
            ty::TyAdt(def, substs) => {
                let did = def.did;
                let kind = match def.adt_kind() {
                    AdtKind::Struct => TypeKind::Struct,
                    AdtKind::Union => TypeKind::Union,
                    AdtKind::Enum => TypeKind::Enum,
                };
                inline::record_extern_fqn(cx, did, kind);
                let path = external_path(cx, &cx.tcx.item_name(did).as_str(),
                                         None, false, vec![], substs);
                ResolvedPath {
                    path,
                    typarams: None,
                    did,
                    is_generic: false,
                }
            }
            ty::TyForeign(did) => {
                inline::record_extern_fqn(cx, did, TypeKind::Foreign);
                let path = external_path(cx, &cx.tcx.item_name(did).as_str(),
                                         None, false, vec![], Substs::empty());
                ResolvedPath {
                    path: path,
                    typarams: None,
                    did: did,
                    is_generic: false,
                }
            }
            ty::TyDynamic(ref obj, ref reg) => {
                if let Some(principal) = obj.principal() {
                    let did = principal.def_id();
                    inline::record_extern_fqn(cx, did, TypeKind::Trait);

                    let mut typarams = vec![];
                    reg.clean(cx).map(|b| typarams.push(RegionBound(b)));
                    for did in obj.auto_traits() {
                        let empty = cx.tcx.intern_substs(&[]);
                        let path = external_path(cx, &cx.tcx.item_name(did).as_str(),
                            Some(did), false, vec![], empty);
                        inline::record_extern_fqn(cx, did, TypeKind::Trait);
                        let bound = TraitBound(PolyTrait {
                            trait_: ResolvedPath {
                                path,
                                typarams: None,
                                did,
                                is_generic: false,
                            },
                            generic_params: Vec::new(),
                        }, hir::TraitBoundModifier::None);
                        typarams.push(bound);
                    }

                    let mut bindings = vec![];
                    for pb in obj.projection_bounds() {
                        bindings.push(TypeBinding {
                            name: cx.tcx.associated_item(pb.item_def_id()).name.clean(cx),
                            ty: pb.skip_binder().ty.clean(cx)
                        });
                    }

                    let path = external_path(cx, &cx.tcx.item_name(did).as_str(), Some(did),
                        false, bindings, principal.skip_binder().substs);
                    ResolvedPath {
                        path,
                        typarams: Some(typarams),
                        did,
                        is_generic: false,
                    }
                } else {
                    Never
                }
            }
            ty::TyTuple(ref t) => Tuple(t.clean(cx)),

            ty::TyProjection(ref data) => data.clean(cx),

            ty::TyParam(ref p) => Generic(p.name.to_string()),

            ty::TyAnon(def_id, substs) => {
                // Grab the "TraitA + TraitB" from `impl TraitA + TraitB`,
                // by looking up the projections associated with the def_id.
                let predicates_of = cx.tcx.predicates_of(def_id);
                let substs = cx.tcx.lift(&substs).unwrap();
                let bounds = predicates_of.instantiate(cx.tcx, substs);
                let mut regions = vec![];
                let mut has_sized = false;
                let mut bounds = bounds.predicates.iter().filter_map(|predicate| {
                    let trait_ref = if let Some(tr) = predicate.to_opt_poly_trait_ref() {
                        tr
                    } else if let ty::Predicate::TypeOutlives(pred) = *predicate {
                        // these should turn up at the end
                        pred.skip_binder().1.clean(cx).map(|r| regions.push(RegionBound(r)));
                        return None;
                    } else {
                        return None;
                    };

                    if let Some(sized) = cx.tcx.lang_items().sized_trait() {
                        if trait_ref.def_id() == sized {
                            has_sized = true;
                            return None;
                        }
                    }


                    let bounds = bounds.predicates.iter().filter_map(|pred|
                        if let ty::Predicate::Projection(proj) = *pred {
                            let proj = proj.skip_binder();
                            if proj.projection_ty.trait_ref(cx.tcx) == *trait_ref.skip_binder() {
                                Some(TypeBinding {
                                    name: cx.tcx.associated_item(proj.projection_ty.item_def_id)
                                                .name.clean(cx),
                                    ty: proj.ty.clean(cx),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    ).collect();

                    Some((trait_ref.skip_binder(), bounds).clean(cx))
                }).collect::<Vec<_>>();
                bounds.extend(regions);
                if !has_sized && !bounds.is_empty() {
                    bounds.insert(0, TyParamBound::maybe_sized(cx));
                }
                ImplTrait(bounds)
            }

            ty::TyClosure(..) | ty::TyGenerator(..) => Tuple(vec![]), // FIXME(pcwalton)

            ty::TyGeneratorWitness(..) => panic!("TyGeneratorWitness"),
            ty::TyInfer(..) => panic!("TyInfer"),
            ty::TyError => panic!("TyError"),
        }
    }
}

impl Clean<Item> for hir::StructField {
    fn clean(&self, cx: &DocContext) -> Item {
        Item {
            name: Some(self.ident.name).clean(cx),
            attrs: self.attrs.clean(cx),
            source: self.span.clean(cx),
            visibility: self.vis.clean(cx),
            stability: get_stability(cx, cx.tcx.hir.local_def_id(self.id)),
            deprecation: get_deprecation(cx, cx.tcx.hir.local_def_id(self.id)),
            def_id: cx.tcx.hir.local_def_id(self.id),
            inner: StructFieldItem(self.ty.clean(cx)),
        }
    }
}

impl<'tcx> Clean<Item> for ty::FieldDef {
    fn clean(&self, cx: &DocContext) -> Item {
        Item {
            name: Some(self.ident.name).clean(cx),
            attrs: cx.tcx.get_attrs(self.did).clean(cx),
            source: cx.tcx.def_span(self.did).clean(cx),
            visibility: self.vis.clean(cx),
            stability: get_stability(cx, self.did),
            deprecation: get_deprecation(cx, self.did),
            def_id: self.did,
            inner: StructFieldItem(cx.tcx.type_of(self.did).clean(cx)),
        }
    }
}

#[derive(Clone, PartialEq, Eq, RustcDecodable, RustcEncodable, Debug)]
pub enum Visibility {
    Public,
    Inherited,
    Crate,
    Restricted(DefId, Path),
}

impl Clean<Option<Visibility>> for hir::Visibility {
    fn clean(&self, cx: &DocContext) -> Option<Visibility> {
        Some(match *self {
            hir::Visibility::Public => Visibility::Public,
            hir::Visibility::Inherited => Visibility::Inherited,
            hir::Visibility::Crate(_) => Visibility::Crate,
            hir::Visibility::Restricted { ref path, .. } => {
                let path = path.clean(cx);
                let did = register_def(cx, path.def);
                Visibility::Restricted(did, path)
            }
        })
    }
}

impl Clean<Option<Visibility>> for ty::Visibility {
    fn clean(&self, _: &DocContext) -> Option<Visibility> {
        Some(if *self == ty::Visibility::Public { Public } else { Inherited })
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Struct {
    pub struct_type: doctree::StructType,
    pub generics: Generics,
    pub fields: Vec<Item>,
    pub fields_stripped: bool,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Union {
    pub struct_type: doctree::StructType,
    pub generics: Generics,
    pub fields: Vec<Item>,
    pub fields_stripped: bool,
}

impl Clean<Vec<Item>> for doctree::Struct {
    fn clean(&self, cx: &DocContext) -> Vec<Item> {
        let name = self.name.clean(cx);
        let mut ret = get_auto_traits_with_node_id(cx, self.id, name.clone());

        ret.push(Item {
            name: Some(name),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: StructItem(Struct {
                struct_type: self.struct_type,
                generics: self.generics.clean(cx),
                fields: self.fields.clean(cx),
                fields_stripped: false,
            }),
        });

        ret
    }
}

impl Clean<Vec<Item>> for doctree::Union {
    fn clean(&self, cx: &DocContext) -> Vec<Item> {
        let name = self.name.clean(cx);
        let mut ret = get_auto_traits_with_node_id(cx, self.id, name.clone());

        ret.push(Item {
            name: Some(name),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: UnionItem(Union {
                struct_type: self.struct_type,
                generics: self.generics.clean(cx),
                fields: self.fields.clean(cx),
                fields_stripped: false,
            }),
        });

        ret
    }
}

/// This is a more limited form of the standard Struct, different in that
/// it lacks the things most items have (name, id, parameterization). Found
/// only as a variant in an enum.
#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct VariantStruct {
    pub struct_type: doctree::StructType,
    pub fields: Vec<Item>,
    pub fields_stripped: bool,
}

impl Clean<VariantStruct> for ::rustc::hir::VariantData {
    fn clean(&self, cx: &DocContext) -> VariantStruct {
        VariantStruct {
            struct_type: doctree::struct_type_from_def(self),
            fields: self.fields().iter().map(|x| x.clean(cx)).collect(),
            fields_stripped: false,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Enum {
    pub variants: Vec<Item>,
    pub generics: Generics,
    pub variants_stripped: bool,
}

impl Clean<Vec<Item>> for doctree::Enum {
    fn clean(&self, cx: &DocContext) -> Vec<Item> {
        let name = self.name.clean(cx);
        let mut ret = get_auto_traits_with_node_id(cx, self.id, name.clone());

        ret.push(Item {
            name: Some(name),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: EnumItem(Enum {
                variants: self.variants.clean(cx),
                generics: self.generics.clean(cx),
                variants_stripped: false,
            }),
        });

        ret
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Variant {
    pub kind: VariantKind,
}

impl Clean<Item> for doctree::Variant {
    fn clean(&self, cx: &DocContext) -> Item {
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            visibility: None,
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.def.id()),
            inner: VariantItem(Variant {
                kind: self.def.clean(cx),
            }),
        }
    }
}

impl<'tcx> Clean<Item> for ty::VariantDef {
    fn clean(&self, cx: &DocContext) -> Item {
        let kind = match self.ctor_kind {
            CtorKind::Const => VariantKind::CLike,
            CtorKind::Fn => {
                VariantKind::Tuple(
                    self.fields.iter().map(|f| cx.tcx.type_of(f.did).clean(cx)).collect()
                )
            }
            CtorKind::Fictive => {
                VariantKind::Struct(VariantStruct {
                    struct_type: doctree::Plain,
                    fields_stripped: false,
                    fields: self.fields.iter().map(|field| {
                        Item {
                            source: cx.tcx.def_span(field.did).clean(cx),
                            name: Some(field.ident.name.clean(cx)),
                            attrs: cx.tcx.get_attrs(field.did).clean(cx),
                            visibility: field.vis.clean(cx),
                            def_id: field.did,
                            stability: get_stability(cx, field.did),
                            deprecation: get_deprecation(cx, field.did),
                            inner: StructFieldItem(cx.tcx.type_of(field.did).clean(cx))
                        }
                    }).collect()
                })
            }
        };
        Item {
            name: Some(self.name.clean(cx)),
            attrs: inline::load_attrs(cx, self.did),
            source: cx.tcx.def_span(self.did).clean(cx),
            visibility: Some(Inherited),
            def_id: self.did,
            inner: VariantItem(Variant { kind: kind }),
            stability: get_stability(cx, self.did),
            deprecation: get_deprecation(cx, self.did),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub enum VariantKind {
    CLike,
    Tuple(Vec<Type>),
    Struct(VariantStruct),
}

impl Clean<VariantKind> for hir::VariantData {
    fn clean(&self, cx: &DocContext) -> VariantKind {
        if self.is_struct() {
            VariantKind::Struct(self.clean(cx))
        } else if self.is_unit() {
            VariantKind::CLike
        } else {
            VariantKind::Tuple(self.fields().iter().map(|x| x.ty.clean(cx)).collect())
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Span {
    pub filename: FileName,
    pub loline: usize,
    pub locol: usize,
    pub hiline: usize,
    pub hicol: usize,
}

impl Span {
    pub fn empty() -> Span {
        Span {
            filename: FileName::Anon,
            loline: 0, locol: 0,
            hiline: 0, hicol: 0,
        }
    }
}

impl Clean<Span> for syntax_pos::Span {
    fn clean(&self, cx: &DocContext) -> Span {
        if *self == DUMMY_SP {
            return Span::empty();
        }

        let cm = cx.sess().codemap();
        let filename = cm.span_to_filename(*self);
        let lo = cm.lookup_char_pos(self.lo());
        let hi = cm.lookup_char_pos(self.hi());
        Span {
            filename,
            loline: lo.line,
            locol: lo.col.to_usize(),
            hiline: hi.line,
            hicol: hi.col.to_usize(),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct Path {
    pub global: bool,
    pub def: Def,
    pub segments: Vec<PathSegment>,
}

impl Path {
    pub fn singleton(name: String) -> Path {
        Path {
            global: false,
            def: Def::Err,
            segments: vec![PathSegment {
                name,
                params: PathParameters::AngleBracketed {
                    lifetimes: Vec::new(),
                    types: Vec::new(),
                    bindings: Vec::new(),
                }
            }]
        }
    }

    pub fn last_name(&self) -> &str {
        self.segments.last().unwrap().name.as_str()
    }
}

impl Clean<Path> for hir::Path {
    fn clean(&self, cx: &DocContext) -> Path {
        Path {
            global: self.is_global(),
            def: self.def,
            segments: if self.is_global() { &self.segments[1..] } else { &self.segments }.clean(cx),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub enum PathParameters {
    AngleBracketed {
        lifetimes: Vec<Lifetime>,
        types: Vec<Type>,
        bindings: Vec<TypeBinding>,
    },
    Parenthesized {
        inputs: Vec<Type>,
        output: Option<Type>,
    }
}

impl Clean<PathParameters> for hir::PathParameters {
    fn clean(&self, cx: &DocContext) -> PathParameters {
        if self.parenthesized {
            let output = self.bindings[0].ty.clean(cx);
            PathParameters::Parenthesized {
                inputs: self.inputs().clean(cx),
                output: if output != Type::Tuple(Vec::new()) { Some(output) } else { None }
            }
        } else {
            PathParameters::AngleBracketed {
                lifetimes: if self.lifetimes.iter().all(|lt| lt.is_elided()) {
                    vec![]
                } else {
                    self.lifetimes.clean(cx)
                },
                types: self.types.clean(cx),
                bindings: self.bindings.clean(cx),
            }
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct PathSegment {
    pub name: String,
    pub params: PathParameters,
}

impl Clean<PathSegment> for hir::PathSegment {
    fn clean(&self, cx: &DocContext) -> PathSegment {
        PathSegment {
            name: self.name.clean(cx),
            params: self.with_parameters(|parameters| parameters.clean(cx))
        }
    }
}

fn strip_type(ty: Type) -> Type {
    match ty {
        Type::ResolvedPath { path, typarams, did, is_generic } => {
            Type::ResolvedPath { path: strip_path(&path), typarams, did, is_generic }
        }
        Type::Tuple(inner_tys) => {
            Type::Tuple(inner_tys.iter().map(|t| strip_type(t.clone())).collect())
        }
        Type::Slice(inner_ty) => Type::Slice(Box::new(strip_type(*inner_ty))),
        Type::Array(inner_ty, s) => Type::Array(Box::new(strip_type(*inner_ty)), s),
        Type::Unique(inner_ty) => Type::Unique(Box::new(strip_type(*inner_ty))),
        Type::RawPointer(m, inner_ty) => Type::RawPointer(m, Box::new(strip_type(*inner_ty))),
        Type::BorrowedRef { lifetime, mutability, type_ } => {
            Type::BorrowedRef { lifetime, mutability, type_: Box::new(strip_type(*type_)) }
        }
        Type::QPath { name, self_type, trait_ } => {
            Type::QPath {
                name,
                self_type: Box::new(strip_type(*self_type)), trait_: Box::new(strip_type(*trait_))
            }
        }
        _ => ty
    }
}

fn strip_path(path: &Path) -> Path {
    let segments = path.segments.iter().map(|s| {
        PathSegment {
            name: s.name.clone(),
            params: PathParameters::AngleBracketed {
                lifetimes: Vec::new(),
                types: Vec::new(),
                bindings: Vec::new(),
            }
        }
    }).collect();

    Path {
        global: path.global,
        def: path.def.clone(),
        segments,
    }
}

fn qpath_to_string(p: &hir::QPath) -> String {
    let segments = match *p {
        hir::QPath::Resolved(_, ref path) => &path.segments,
        hir::QPath::TypeRelative(_, ref segment) => return segment.name.to_string(),
    };

    let mut s = String::new();
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            s.push_str("::");
        }
        if seg.name != keywords::CrateRoot.name() {
            s.push_str(&*seg.name.as_str());
        }
    }
    s
}

impl Clean<String> for ast::Name {
    fn clean(&self, _: &DocContext) -> String {
        self.to_string()
    }
}

impl Clean<String> for InternedString {
    fn clean(&self, _: &DocContext) -> String {
        self.to_string()
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Typedef {
    pub type_: Type,
    pub generics: Generics,
}

impl Clean<Item> for doctree::Typedef {
    fn clean(&self, cx: &DocContext) -> Item {
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id.clone()),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: TypedefItem(Typedef {
                type_: self.ty.clean(cx),
                generics: self.gen.clean(cx),
            }, false),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Debug, Hash)]
pub struct BareFunctionDecl {
    pub unsafety: hir::Unsafety,
    pub generic_params: Vec<GenericParamDef>,
    pub decl: FnDecl,
    pub abi: Abi,
}

impl Clean<BareFunctionDecl> for hir::BareFnTy {
    fn clean(&self, cx: &DocContext) -> BareFunctionDecl {
        let (generic_params, decl) = enter_impl_trait(cx, || {
            (self.generic_params.clean(cx), (&*self.decl, &self.arg_names[..]).clean(cx))
        });
        BareFunctionDecl {
            unsafety: self.unsafety,
            decl,
            generic_params,
            abi: self.abi,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Static {
    pub type_: Type,
    pub mutability: Mutability,
    /// It's useful to have the value of a static documented, but I have no
    /// desire to represent expressions (that'd basically be all of the AST,
    /// which is huge!). So, have a string.
    pub expr: String,
}

impl Clean<Item> for doctree::Static {
    fn clean(&self, cx: &DocContext) -> Item {
        debug!("cleaning static {}: {:?}", self.name.clean(cx), self);
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: StaticItem(Static {
                type_: self.type_.clean(cx),
                mutability: self.mutability.clean(cx),
                expr: print_const_expr(cx, self.expr),
            }),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Constant {
    pub type_: Type,
    pub expr: String,
}

impl Clean<Item> for doctree::Constant {
    fn clean(&self, cx: &DocContext) -> Item {
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: ConstantItem(Constant {
                type_: self.type_.clean(cx),
                expr: print_const_expr(cx, self.expr),
            }),
        }
    }
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Copy, Hash)]
pub enum Mutability {
    Mutable,
    Immutable,
}

impl Clean<Mutability> for hir::Mutability {
    fn clean(&self, _: &DocContext) -> Mutability {
        match self {
            &hir::MutMutable => Mutable,
            &hir::MutImmutable => Immutable,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Copy, Debug, Hash)]
pub enum ImplPolarity {
    Positive,
    Negative,
}

impl Clean<ImplPolarity> for hir::ImplPolarity {
    fn clean(&self, _: &DocContext) -> ImplPolarity {
        match self {
            &hir::ImplPolarity::Positive => ImplPolarity::Positive,
            &hir::ImplPolarity::Negative => ImplPolarity::Negative,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Impl {
    pub unsafety: hir::Unsafety,
    pub generics: Generics,
    pub provided_trait_methods: FxHashSet<String>,
    pub trait_: Option<Type>,
    pub for_: Type,
    pub items: Vec<Item>,
    pub polarity: Option<ImplPolarity>,
    pub synthetic: bool,
}

pub fn get_auto_traits_with_node_id(cx: &DocContext, id: ast::NodeId, name: String) -> Vec<Item> {
    let finder = AutoTraitFinder::new(cx);
    finder.get_with_node_id(id, name)
}

pub fn get_auto_traits_with_def_id(cx: &DocContext, id: DefId) -> Vec<Item> {
    let finder = AutoTraitFinder::new(cx);

    finder.get_with_def_id(id)
}

impl Clean<Vec<Item>> for doctree::Impl {
    fn clean(&self, cx: &DocContext) -> Vec<Item> {
        let mut ret = Vec::new();
        let trait_ = self.trait_.clean(cx);
        let items = self.items.clean(cx);

        // If this impl block is an implementation of the Deref trait, then we
        // need to try inlining the target's inherent impl blocks as well.
        if trait_.def_id() == cx.tcx.lang_items().deref_trait() {
            build_deref_target_impls(cx, &items, &mut ret);
        }

        let provided = trait_.def_id().map(|did| {
            cx.tcx.provided_trait_methods(did)
                  .into_iter()
                  .map(|meth| meth.name.to_string())
                  .collect()
        }).unwrap_or(FxHashSet());

        ret.push(Item {
            name: None,
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            inner: ImplItem(Impl {
                unsafety: self.unsafety,
                generics: self.generics.clean(cx),
                provided_trait_methods: provided,
                trait_,
                for_: self.for_.clean(cx),
                items,
                polarity: Some(self.polarity.clean(cx)),
                synthetic: false,
            })
        });
        ret
    }
}

fn build_deref_target_impls(cx: &DocContext,
                            items: &[Item],
                            ret: &mut Vec<Item>) {
    use self::PrimitiveType::*;
    let tcx = cx.tcx;

    for item in items {
        let target = match item.inner {
            TypedefItem(ref t, true) => &t.type_,
            _ => continue,
        };
        let primitive = match *target {
            ResolvedPath { did, .. } if did.is_local() => continue,
            ResolvedPath { did, .. } => {
                // We set the last parameter to false to avoid looking for auto-impls for traits
                // and therefore avoid an ICE.
                // The reason behind this is that auto-traits don't propagate through Deref so
                // we're not supposed to synthesise impls for them.
                ret.extend(inline::build_impls(cx, did, false));
                continue
            }
            _ => match target.primitive_type() {
                Some(prim) => prim,
                None => continue,
            }
        };
        let did = match primitive {
            Isize => tcx.lang_items().isize_impl(),
            I8 => tcx.lang_items().i8_impl(),
            I16 => tcx.lang_items().i16_impl(),
            I32 => tcx.lang_items().i32_impl(),
            I64 => tcx.lang_items().i64_impl(),
            I128 => tcx.lang_items().i128_impl(),
            Usize => tcx.lang_items().usize_impl(),
            U8 => tcx.lang_items().u8_impl(),
            U16 => tcx.lang_items().u16_impl(),
            U32 => tcx.lang_items().u32_impl(),
            U64 => tcx.lang_items().u64_impl(),
            U128 => tcx.lang_items().u128_impl(),
            F32 => tcx.lang_items().f32_impl(),
            F64 => tcx.lang_items().f64_impl(),
            Char => tcx.lang_items().char_impl(),
            Bool => None,
            Str => tcx.lang_items().str_impl(),
            Slice => tcx.lang_items().slice_impl(),
            Array => tcx.lang_items().slice_impl(),
            Tuple => None,
            Unit => None,
            RawPointer => tcx.lang_items().const_ptr_impl(),
            Reference => None,
            Fn => None,
            Never => None,
        };
        if let Some(did) = did {
            if !did.is_local() {
                inline::build_impl(cx, did, ret);
            }
        }
    }
}

impl Clean<Item> for doctree::ExternCrate {
    fn clean(&self, cx: &DocContext) -> Item {
        Item {
            name: None,
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: DefId { krate: self.cnum, index: CRATE_DEF_INDEX },
            visibility: self.vis.clean(cx),
            stability: None,
            deprecation: None,
            inner: ExternCrateItem(self.name.clean(cx), self.path.clone())
        }
    }
}

impl Clean<Vec<Item>> for doctree::Import {
    fn clean(&self, cx: &DocContext) -> Vec<Item> {
        // We consider inlining the documentation of `pub use` statements, but we
        // forcefully don't inline if this is not public or if the
        // #[doc(no_inline)] attribute is present.
        // Don't inline doc(hidden) imports so they can be stripped at a later stage.
        let denied = self.vis != hir::Public || self.attrs.iter().any(|a| {
            a.name() == "doc" && match a.meta_item_list() {
                Some(l) => attr::list_contains_name(&l, "no_inline") ||
                           attr::list_contains_name(&l, "hidden"),
                None => false,
            }
        });
        let path = self.path.clean(cx);
        let inner = if self.glob {
            if !denied {
                let mut visited = FxHashSet();
                if let Some(items) = inline::try_inline_glob(cx, path.def, &mut visited) {
                    return items;
                }
            }

            Import::Glob(resolve_use_source(cx, path))
        } else {
            let name = self.name;
            if !denied {
                let mut visited = FxHashSet();
                if let Some(items) = inline::try_inline(cx, path.def, name, &mut visited) {
                    return items;
                }
            }
            Import::Simple(name.clean(cx), resolve_use_source(cx, path))
        };
        vec![Item {
            name: None,
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            def_id: cx.tcx.hir.local_def_id(ast::CRATE_NODE_ID),
            visibility: self.vis.clean(cx),
            stability: None,
            deprecation: None,
            inner: ImportItem(inner)
        }]
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub enum Import {
    // use source as str;
    Simple(String, ImportSource),
    // use source::*;
    Glob(ImportSource)
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct ImportSource {
    pub path: Path,
    pub did: Option<DefId>,
}

impl Clean<Vec<Item>> for hir::ForeignMod {
    fn clean(&self, cx: &DocContext) -> Vec<Item> {
        let mut items = self.items.clean(cx);
        for item in &mut items {
            if let ForeignFunctionItem(ref mut f) = item.inner {
                f.abi = self.abi;
            }
        }
        items
    }
}

impl Clean<Item> for hir::ForeignItem {
    fn clean(&self, cx: &DocContext) -> Item {
        let inner = match self.node {
            hir::ForeignItemFn(ref decl, ref names, ref generics) => {
                let (generics, decl) = enter_impl_trait(cx, || {
                    (generics.clean(cx), (&**decl, &names[..]).clean(cx))
                });
                ForeignFunctionItem(Function {
                    decl,
                    generics,
                    unsafety: hir::Unsafety::Unsafe,
                    abi: Abi::Rust,
                    constness: hir::Constness::NotConst,
                })
            }
            hir::ForeignItemStatic(ref ty, mutbl) => {
                ForeignStaticItem(Static {
                    type_: ty.clean(cx),
                    mutability: if mutbl {Mutable} else {Immutable},
                    expr: "".to_string(),
                })
            }
            hir::ForeignItemType => {
                ForeignTypeItem
            }
        };
        Item {
            name: Some(self.name.clean(cx)),
            attrs: self.attrs.clean(cx),
            source: self.span.clean(cx),
            def_id: cx.tcx.hir.local_def_id(self.id),
            visibility: self.vis.clean(cx),
            stability: get_stability(cx, cx.tcx.hir.local_def_id(self.id)),
            deprecation: get_deprecation(cx, cx.tcx.hir.local_def_id(self.id)),
            inner,
        }
    }
}

// Utilities

trait ToSource {
    fn to_src(&self, cx: &DocContext) -> String;
}

impl ToSource for syntax_pos::Span {
    fn to_src(&self, cx: &DocContext) -> String {
        debug!("converting span {:?} to snippet", self.clean(cx));
        let sn = match cx.sess().codemap().span_to_snippet(*self) {
            Ok(x) => x.to_string(),
            Err(_) => "".to_string()
        };
        debug!("got snippet {}", sn);
        sn
    }
}

fn name_from_pat(p: &hir::Pat) -> String {
    use rustc::hir::*;
    debug!("Trying to get a name from pattern: {:?}", p);

    match p.node {
        PatKind::Wild => "_".to_string(),
        PatKind::Binding(_, _, ref p, _) => p.node.to_string(),
        PatKind::TupleStruct(ref p, ..) | PatKind::Path(ref p) => qpath_to_string(p),
        PatKind::Struct(ref name, ref fields, etc) => {
            format!("{} {{ {}{} }}", qpath_to_string(name),
                fields.iter().map(|&Spanned { node: ref fp, .. }|
                                  format!("{}: {}", fp.ident, name_from_pat(&*fp.pat)))
                             .collect::<Vec<String>>().join(", "),
                if etc { ", ..." } else { "" }
            )
        }
        PatKind::Tuple(ref elts, _) => format!("({})", elts.iter().map(|p| name_from_pat(&**p))
                                            .collect::<Vec<String>>().join(", ")),
        PatKind::Box(ref p) => name_from_pat(&**p),
        PatKind::Ref(ref p, _) => name_from_pat(&**p),
        PatKind::Lit(..) => {
            warn!("tried to get argument name from PatKind::Lit, \
                  which is silly in function arguments");
            "()".to_string()
        },
        PatKind::Range(..) => panic!("tried to get argument name from PatKind::Range, \
                              which is not allowed in function arguments"),
        PatKind::Slice(ref begin, ref mid, ref end) => {
            let begin = begin.iter().map(|p| name_from_pat(&**p));
            let mid = mid.as_ref().map(|p| format!("..{}", name_from_pat(&**p))).into_iter();
            let end = end.iter().map(|p| name_from_pat(&**p));
            format!("[{}]", begin.chain(mid).chain(end).collect::<Vec<_>>().join(", "))
        },
    }
}

fn print_const(cx: &DocContext, n: &ty::Const) -> String {
    match n.val {
        ConstVal::Unevaluated(def_id, _) => {
            if let Some(node_id) = cx.tcx.hir.as_local_node_id(def_id) {
                print_const_expr(cx, cx.tcx.hir.body_owned_by(node_id))
            } else {
                inline::print_inlined_const(cx, def_id)
            }
        },
        ConstVal::Value(..) => {
            let mut s = String::new();
            ::rustc::mir::fmt_const_val(&mut s, n).unwrap();
            // array lengths are obviously usize
            if s.ends_with("usize") {
                let n = s.len() - "usize".len();
                s.truncate(n);
            }
            s
        },
    }
}

fn print_const_expr(cx: &DocContext, body: hir::BodyId) -> String {
    cx.tcx.hir.node_to_pretty_string(body.node_id)
}

/// Given a type Path, resolve it to a Type using the TyCtxt
fn resolve_type(cx: &DocContext,
                path: Path,
                id: ast::NodeId) -> Type {
    if id == ast::DUMMY_NODE_ID {
        debug!("resolve_type({:?})", path);
    } else {
        debug!("resolve_type({:?},{:?})", path, id);
    }

    let is_generic = match path.def {
        Def::PrimTy(p) => match p {
            hir::TyStr => return Primitive(PrimitiveType::Str),
            hir::TyBool => return Primitive(PrimitiveType::Bool),
            hir::TyChar => return Primitive(PrimitiveType::Char),
            hir::TyInt(int_ty) => return Primitive(int_ty.into()),
            hir::TyUint(uint_ty) => return Primitive(uint_ty.into()),
            hir::TyFloat(float_ty) => return Primitive(float_ty.into()),
        },
        Def::SelfTy(..) if path.segments.len() == 1 => {
            return Generic(keywords::SelfType.name().to_string());
        }
        Def::TyParam(..) if path.segments.len() == 1 => {
            return Generic(format!("{:#}", path));
        }
        Def::SelfTy(..) | Def::TyParam(..) | Def::AssociatedTy(..) => true,
        _ => false,
    };
    let did = register_def(&*cx, path.def);
    ResolvedPath { path: path, typarams: None, did: did, is_generic: is_generic }
}

fn register_def(cx: &DocContext, def: Def) -> DefId {
    debug!("register_def({:?})", def);

    let (did, kind) = match def {
        Def::Fn(i) => (i, TypeKind::Function),
        Def::TyAlias(i) => (i, TypeKind::Typedef),
        Def::Enum(i) => (i, TypeKind::Enum),
        Def::Trait(i) => (i, TypeKind::Trait),
        Def::Struct(i) => (i, TypeKind::Struct),
        Def::Union(i) => (i, TypeKind::Union),
        Def::Mod(i) => (i, TypeKind::Module),
        Def::TyForeign(i) => (i, TypeKind::Foreign),
        Def::Const(i) => (i, TypeKind::Const),
        Def::Static(i, _) => (i, TypeKind::Static),
        Def::Variant(i) => (cx.tcx.parent_def_id(i).unwrap(), TypeKind::Enum),
        Def::Macro(i, _) => (i, TypeKind::Macro),
        Def::SelfTy(Some(def_id), _) => (def_id, TypeKind::Trait),
        Def::SelfTy(_, Some(impl_def_id)) => {
            return impl_def_id
        }
        _ => return def.def_id()
    };
    if did.is_local() { return did }
    inline::record_extern_fqn(cx, did, kind);
    if let TypeKind::Trait = kind {
        inline::record_extern_trait(cx, did);
    }
    did
}

fn resolve_use_source(cx: &DocContext, path: Path) -> ImportSource {
    ImportSource {
        did: if path.def == Def::Err {
            None
        } else {
            Some(register_def(cx, path.def))
        },
        path,
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Macro {
    pub source: String,
    pub imported_from: Option<String>,
}

impl Clean<Item> for doctree::Macro {
    fn clean(&self, cx: &DocContext) -> Item {
        let name = self.name.clean(cx);
        Item {
            name: Some(name.clone()),
            attrs: self.attrs.clean(cx),
            source: self.whence.clean(cx),
            visibility: Some(Public),
            stability: self.stab.clean(cx),
            deprecation: self.depr.clean(cx),
            def_id: self.def_id,
            inner: MacroItem(Macro {
                source: format!("macro_rules! {} {{\n{}}}",
                                name,
                                self.matchers.iter().map(|span| {
                                    format!("    {} => {{ ... }};\n", span.to_src(cx))
                                }).collect::<String>()),
                imported_from: self.imported_from.clean(cx),
            }),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Stability {
    pub level: stability::StabilityLevel,
    pub feature: String,
    pub since: String,
    pub deprecated_since: String,
    pub deprecated_reason: String,
    pub unstable_reason: String,
    pub issue: Option<u32>
}

#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Deprecation {
    pub since: String,
    pub note: String,
}

impl Clean<Stability> for attr::Stability {
    fn clean(&self, _: &DocContext) -> Stability {
        Stability {
            level: stability::StabilityLevel::from_attr_level(&self.level),
            feature: self.feature.to_string(),
            since: match self.level {
                attr::Stable {ref since} => since.to_string(),
                _ => "".to_string(),
            },
            deprecated_since: match self.rustc_depr {
                Some(attr::RustcDeprecation {ref since, ..}) => since.to_string(),
                _=> "".to_string(),
            },
            deprecated_reason: match self.rustc_depr {
                Some(ref depr) => depr.reason.to_string(),
                _ => "".to_string(),
            },
            unstable_reason: match self.level {
                attr::Unstable { reason: Some(ref reason), .. } => reason.to_string(),
                _ => "".to_string(),
            },
            issue: match self.level {
                attr::Unstable {issue, ..} => Some(issue),
                _ => None,
            }
        }
    }
}

impl<'a> Clean<Stability> for &'a attr::Stability {
    fn clean(&self, dc: &DocContext) -> Stability {
        (**self).clean(dc)
    }
}

impl Clean<Deprecation> for attr::Deprecation {
    fn clean(&self, _: &DocContext) -> Deprecation {
        Deprecation {
            since: self.since.as_ref().map_or("".to_string(), |s| s.to_string()),
            note: self.note.as_ref().map_or("".to_string(), |s| s.to_string()),
        }
    }
}

/// An equality constraint on an associated type, e.g. `A=Bar` in `Foo<A=Bar>`
#[derive(Clone, PartialEq, Eq, RustcDecodable, RustcEncodable, Debug, Hash)]
pub struct TypeBinding {
    pub name: String,
    pub ty: Type
}

impl Clean<TypeBinding> for hir::TypeBinding {
    fn clean(&self, cx: &DocContext) -> TypeBinding {
        TypeBinding {
            name: self.name.clean(cx),
            ty: self.ty.clean(cx)
        }
    }
}

pub fn def_id_to_path(cx: &DocContext, did: DefId, name: Option<String>) -> Vec<String> {
    let crate_name = name.unwrap_or_else(|| cx.tcx.crate_name(did.krate).to_string());
    let relative = cx.tcx.def_path(did).data.into_iter().filter_map(|elem| {
        // extern blocks have an empty name
        let s = elem.data.to_string();
        if !s.is_empty() {
            Some(s)
        } else {
            None
        }
    });
    once(crate_name).chain(relative).collect()
}

pub fn enter_impl_trait<F, R>(cx: &DocContext, f: F) -> R
where
    F: FnOnce() -> R,
{
    let old_bounds = mem::replace(&mut *cx.impl_trait_bounds.borrow_mut(), Default::default());
    let r = f();
    assert!(cx.impl_trait_bounds.borrow().is_empty());
    *cx.impl_trait_bounds.borrow_mut() = old_bounds;
    r
}

// Start of code copied from rust-clippy

pub fn get_trait_def_id(tcx: &TyCtxt, path: &[&str], use_local: bool) -> Option<DefId> {
    if use_local {
        path_to_def_local(tcx, path)
    } else {
        path_to_def(tcx, path)
    }
}

pub fn path_to_def_local(tcx: &TyCtxt, path: &[&str]) -> Option<DefId> {
    let krate = tcx.hir.krate();
    let mut items = krate.module.item_ids.clone();
    let mut path_it = path.iter().peekable();

    loop {
        let segment = match path_it.next() {
            Some(segment) => segment,
            None => return None,
        };

        for item_id in mem::replace(&mut items, HirVec::new()).iter() {
            let item = tcx.hir.expect_item(item_id.id);
            if item.name == *segment {
                if path_it.peek().is_none() {
                    return Some(tcx.hir.local_def_id(item_id.id))
                }

                items = match &item.node {
                    &hir::ItemMod(ref m) => m.item_ids.clone(),
                    _ => panic!("Unexpected item {:?} in path {:?} path")
                };
                break;
            }
        }
    }
}

pub fn path_to_def(tcx: &TyCtxt, path: &[&str]) -> Option<DefId> {
    let crates = tcx.crates();

    let krate = crates
        .iter()
        .find(|&&krate| tcx.crate_name(krate) == path[0]);

    if let Some(krate) = krate {
        let krate = DefId {
            krate: *krate,
            index: CRATE_DEF_INDEX,
        };
        let mut items = tcx.item_children(krate);
        let mut path_it = path.iter().skip(1).peekable();

        loop {
            let segment = match path_it.next() {
                Some(segment) => segment,
                None => return None,
            };

            for item in mem::replace(&mut items, Lrc::new(vec![])).iter() {
                if item.ident.name == *segment {
                    if path_it.peek().is_none() {
                        return match item.def {
                            def::Def::Trait(did) => Some(did),
                            _ => None,
                        }
                    }

                    items = tcx.item_children(item.def.def_id());
                    break;
                }
            }
        }
    } else {
        None
    }
}

fn get_path_for_type<F>(tcx: TyCtxt, def_id: DefId, def_ctor: F) -> hir::Path
where F: Fn(DefId) -> Def {
    struct AbsolutePathBuffer {
        names: Vec<String>,
    }

    impl ty::item_path::ItemPathBuffer for AbsolutePathBuffer {
        fn root_mode(&self) -> &ty::item_path::RootMode {
            const ABSOLUTE: &'static ty::item_path::RootMode = &ty::item_path::RootMode::Absolute;
            ABSOLUTE
        }

        fn push(&mut self, text: &str) {
            self.names.push(text.to_owned());
        }
    }

    let mut apb = AbsolutePathBuffer { names: vec![] };

    tcx.push_item_path(&mut apb, def_id);

    hir::Path {
        span: DUMMY_SP,
        def: def_ctor(def_id),
        segments: hir::HirVec::from_vec(apb.names.iter().map(|s| hir::PathSegment {
            name: ast::Name::intern(&s),
            parameters: None,
            infer_types: false,
        }).collect())
    }
}

// End of code copied from rust-clippy


#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
enum RegionTarget<'tcx> {
    Region(Region<'tcx>),
    RegionVid(RegionVid)
}

#[derive(Default, Debug, Clone)]
struct RegionDeps<'tcx> {
    larger: FxHashSet<RegionTarget<'tcx>>,
    smaller: FxHashSet<RegionTarget<'tcx>>
}

#[derive(Eq, PartialEq, Hash, Debug)]
enum SimpleBound {
    RegionBound(Lifetime),
    TraitBound(Vec<PathSegment>, Vec<SimpleBound>, Vec<GenericParamDef>, hir::TraitBoundModifier)
}

enum AutoTraitResult {
    ExplicitImpl,
    PositiveImpl(Generics),
    NegativeImpl,
}

impl AutoTraitResult {
    fn is_auto(&self) -> bool {
        match *self {
            AutoTraitResult::PositiveImpl(_) | AutoTraitResult::NegativeImpl => true,
            _ => false,
        }
    }
}

impl From<TyParamBound> for SimpleBound {
    fn from(bound: TyParamBound) -> Self {
        match bound.clone() {
            TyParamBound::RegionBound(l) => SimpleBound::RegionBound(l),
            TyParamBound::TraitBound(t, mod_) => match t.trait_ {
                Type::ResolvedPath { path, typarams, .. } => {
                    SimpleBound::TraitBound(path.segments,
                                            typarams
                                                .map_or_else(|| Vec::new(), |v| v.iter()
                                                        .map(|p| SimpleBound::from(p.clone()))
                                                        .collect()),
                                            t.generic_params,
                                            mod_)
                }
                _ => panic!("Unexpected bound {:?}", bound),
            }
        }
    }
}
