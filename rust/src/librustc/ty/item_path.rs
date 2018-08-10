// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use hir::map::DefPathData;
use hir::def_id::{CrateNum, DefId, CRATE_DEF_INDEX, LOCAL_CRATE};
use ty::{self, Ty, TyCtxt};
use middle::cstore::{ExternCrate, ExternCrateSource};
use syntax::ast;
use syntax::symbol::Symbol;
use syntax::symbol::LocalInternedString;

use std::cell::Cell;

thread_local! {
    static FORCE_ABSOLUTE: Cell<bool> = Cell::new(false);
    static FORCE_IMPL_FILENAME_LINE: Cell<bool> = Cell::new(false);
}

/// Enforces that item_path_str always returns an absolute path and
/// also enables "type-based" impl paths. This is used when building
/// symbols that contain types, where we want the crate name to be
/// part of the symbol.
pub fn with_forced_absolute_paths<F: FnOnce() -> R, R>(f: F) -> R {
    FORCE_ABSOLUTE.with(|force| {
        let old = force.get();
        force.set(true);
        let result = f();
        force.set(old);
        result
    })
}

/// Force us to name impls with just the filename/line number. We
/// normally try to use types. But at some points, notably while printing
/// cycle errors, this can result in extra or suboptimal error output,
/// so this variable disables that check.
pub fn with_forced_impl_filename_line<F: FnOnce() -> R, R>(f: F) -> R {
    FORCE_IMPL_FILENAME_LINE.with(|force| {
        let old = force.get();
        force.set(true);
        let result = f();
        force.set(old);
        result
    })
}

impl<'a, 'gcx, 'tcx> TyCtxt<'a, 'gcx, 'tcx> {
    /// Returns a string identifying this def-id. This string is
    /// suitable for user output. It is relative to the current crate
    /// root, unless with_forced_absolute_paths was used.
    pub fn item_path_str(self, def_id: DefId) -> String {
        let mode = FORCE_ABSOLUTE.with(|force| {
            if force.get() {
                RootMode::Absolute
            } else {
                RootMode::Local
            }
        });
        let mut buffer = LocalPathBuffer::new(mode);
        self.push_item_path(&mut buffer, def_id);
        buffer.into_string()
    }

    /// Returns a string identifying this local node-id.
    pub fn node_path_str(self, id: ast::NodeId) -> String {
        self.item_path_str(self.hir.local_def_id(id))
    }

    /// Returns a string identifying this def-id. This string is
    /// suitable for user output. It always begins with a crate identifier.
    pub fn absolute_item_path_str(self, def_id: DefId) -> String {
        let mut buffer = LocalPathBuffer::new(RootMode::Absolute);
        self.push_item_path(&mut buffer, def_id);
        buffer.into_string()
    }

    /// Returns the "path" to a particular crate. This can proceed in
    /// various ways, depending on the `root_mode` of the `buffer`.
    /// (See `RootMode` enum for more details.)
    pub fn push_krate_path<T>(self, buffer: &mut T, cnum: CrateNum)
        where T: ItemPathBuffer
    {
        match *buffer.root_mode() {
            RootMode::Local => {
                // In local mode, when we encounter a crate other than
                // LOCAL_CRATE, execution proceeds in one of two ways:
                //
                // 1. for a direct dependency, where user added an
                //    `extern crate` manually, we put the `extern
                //    crate` as the parent. So you wind up with
                //    something relative to the current crate.
                // 2. for an extern inferred from a path or an indirect crate,
                //    where there is no explicit `extern crate`, we just prepend
                //    the crate name.
                //
                // Returns `None` for the local crate.
                if cnum != LOCAL_CRATE {
                    let opt_extern_crate = self.extern_crate(cnum.as_def_id());
                    if let Some(ExternCrate {
                        src: ExternCrateSource::Extern(def_id),
                        direct: true,
                        ..
                    }) = *opt_extern_crate
                    {
                        self.push_item_path(buffer, def_id);
                    } else {
                        buffer.push(&self.crate_name(cnum).as_str());
                    }
                }
            }
            RootMode::Absolute => {
                // In absolute mode, just write the crate name
                // unconditionally.
                buffer.push(&self.original_crate_name(cnum).as_str());
            }
        }
    }

    /// If possible, this pushes a global path resolving to `external_def_id` that is visible
    /// from at least one local module and returns true. If the crate defining `external_def_id` is
    /// declared with an `extern crate`, the path is guaranteed to use the `extern crate`.
    pub fn try_push_visible_item_path<T>(self, buffer: &mut T, external_def_id: DefId) -> bool
        where T: ItemPathBuffer
    {
        let visible_parent_map = self.visible_parent_map(LOCAL_CRATE);

        let (mut cur_def, mut cur_path) = (external_def_id, Vec::<LocalInternedString>::new());
        loop {
            // If `cur_def` is a direct or injected extern crate, push the path to the crate
            // followed by the path to the item within the crate and return.
            if cur_def.index == CRATE_DEF_INDEX {
                match *self.extern_crate(cur_def) {
                    Some(ExternCrate {
                        src: ExternCrateSource::Extern(def_id),
                        direct: true,
                        ..
                    }) => {
                        self.push_item_path(buffer, def_id);
                        cur_path.iter().rev().for_each(|segment| buffer.push(&segment));
                        return true;
                    }
                    None => {
                        buffer.push(&self.crate_name(cur_def.krate).as_str());
                        cur_path.iter().rev().for_each(|segment| buffer.push(&segment));
                        return true;
                    }
                    _ => {},
                }
            }

            let mut cur_def_key = self.def_key(cur_def);

            // For a UnitStruct or TupleStruct we want the name of its parent rather than <unnamed>.
            if let DefPathData::StructCtor = cur_def_key.disambiguated_data.data {
                let parent = DefId {
                    krate: cur_def.krate,
                    index: cur_def_key.parent.expect("DefPathData::StructCtor missing a parent"),
                };

                cur_def_key = self.def_key(parent);
            }

            let data = cur_def_key.disambiguated_data.data;
            let symbol = data.get_opt_name().map(|n| n.as_str()).unwrap_or_else(|| {
                if let DefPathData::CrateRoot = data {  // reexported `extern crate` (#43189)
                    self.original_crate_name(cur_def.krate).as_str()
                } else {
                    Symbol::intern("<unnamed>").as_str()
                }
            });
            cur_path.push(symbol);

            match visible_parent_map.get(&cur_def) {
                Some(&def) => cur_def = def,
                None => return false,
            };
        }
    }

    pub fn push_item_path<T>(self, buffer: &mut T, def_id: DefId)
        where T: ItemPathBuffer
    {
        match *buffer.root_mode() {
            RootMode::Local if !def_id.is_local() =>
                if self.try_push_visible_item_path(buffer, def_id) { return },
            _ => {}
        }

        let key = self.def_key(def_id);
        match key.disambiguated_data.data {
            DefPathData::CrateRoot => {
                assert!(key.parent.is_none());
                self.push_krate_path(buffer, def_id.krate);
            }

            DefPathData::Impl => {
                self.push_impl_path(buffer, def_id);
            }

            // Unclear if there is any value in distinguishing these.
            // Probably eventually (and maybe we would even want
            // finer-grained distinctions, e.g. between enum/struct).
            data @ DefPathData::Misc |
            data @ DefPathData::TypeNs(..) |
            data @ DefPathData::Trait(..) |
            data @ DefPathData::AssocTypeInTrait(..) |
            data @ DefPathData::AssocTypeInImpl(..) |
            data @ DefPathData::ValueNs(..) |
            data @ DefPathData::Module(..) |
            data @ DefPathData::TypeParam(..) |
            data @ DefPathData::LifetimeDef(..) |
            data @ DefPathData::EnumVariant(..) |
            data @ DefPathData::Field(..) |
            data @ DefPathData::AnonConst |
            data @ DefPathData::MacroDef(..) |
            data @ DefPathData::ClosureExpr |
            data @ DefPathData::ExistentialImplTrait |
            data @ DefPathData::UniversalImplTrait |
            data @ DefPathData::GlobalMetaData(..) => {
                let parent_def_id = self.parent_def_id(def_id).unwrap();
                self.push_item_path(buffer, parent_def_id);
                buffer.push(&data.as_interned_str().as_symbol().as_str());
            }
            DefPathData::StructCtor => { // present `X` instead of `X::{{constructor}}`
                let parent_def_id = self.parent_def_id(def_id).unwrap();
                self.push_item_path(buffer, parent_def_id);
            }
        }
    }

    fn push_impl_path<T>(self,
                         buffer: &mut T,
                         impl_def_id: DefId)
        where T: ItemPathBuffer
    {
        let parent_def_id = self.parent_def_id(impl_def_id).unwrap();

        // Always use types for non-local impls, where types are always
        // available, and filename/line-number is mostly uninteresting.
        let use_types = !impl_def_id.is_local() || {
            // Otherwise, use filename/line-number if forced.
            let force_no_types = FORCE_IMPL_FILENAME_LINE.with(|f| f.get());
            !force_no_types
        };

        if !use_types {
            return self.push_impl_path_fallback(buffer, impl_def_id);
        }

        // Decide whether to print the parent path for the impl.
        // Logically, since impls are global, it's never needed, but
        // users may find it useful. Currently, we omit the parent if
        // the impl is either in the same module as the self-type or
        // as the trait.
        let self_ty = self.type_of(impl_def_id);
        let in_self_mod = match characteristic_def_id_of_type(self_ty) {
            None => false,
            Some(ty_def_id) => self.parent_def_id(ty_def_id) == Some(parent_def_id),
        };

        let impl_trait_ref = self.impl_trait_ref(impl_def_id);
        let in_trait_mod = match impl_trait_ref {
            None => false,
            Some(trait_ref) => self.parent_def_id(trait_ref.def_id) == Some(parent_def_id),
        };

        if !in_self_mod && !in_trait_mod {
            // If the impl is not co-located with either self-type or
            // trait-type, then fallback to a format that identifies
            // the module more clearly.
            self.push_item_path(buffer, parent_def_id);
            if let Some(trait_ref) = impl_trait_ref {
                buffer.push(&format!("<impl {} for {}>", trait_ref, self_ty));
            } else {
                buffer.push(&format!("<impl {}>", self_ty));
            }
            return;
        }

        // Otherwise, try to give a good form that would be valid language
        // syntax. Preferably using associated item notation.

        if let Some(trait_ref) = impl_trait_ref {
            // Trait impls.
            buffer.push(&format!("<{} as {}>",
                                 self_ty,
                                 trait_ref));
            return;
        }

        // Inherent impls. Try to print `Foo::bar` for an inherent
        // impl on `Foo`, but fallback to `<Foo>::bar` if self-type is
        // anything other than a simple path.
        match self_ty.sty {
            ty::TyAdt(adt_def, substs) => {
                if substs.types().next().is_none() { // ignore regions
                    self.push_item_path(buffer, adt_def.did);
                } else {
                    buffer.push(&format!("<{}>", self_ty));
                }
            }

            ty::TyForeign(did) => self.push_item_path(buffer, did),

            ty::TyBool |
            ty::TyChar |
            ty::TyInt(_) |
            ty::TyUint(_) |
            ty::TyFloat(_) |
            ty::TyStr => {
                buffer.push(&format!("{}", self_ty));
            }

            _ => {
                buffer.push(&format!("<{}>", self_ty));
            }
        }
    }

    fn push_impl_path_fallback<T>(self,
                                  buffer: &mut T,
                                  impl_def_id: DefId)
        where T: ItemPathBuffer
    {
        // If no type info is available, fall back to
        // pretty printing some span information. This should
        // only occur very early in the compiler pipeline.
        let parent_def_id = self.parent_def_id(impl_def_id).unwrap();
        self.push_item_path(buffer, parent_def_id);
        let node_id = self.hir.as_local_node_id(impl_def_id).unwrap();
        let item = self.hir.expect_item(node_id);
        let span_str = self.sess.codemap().span_to_string(item.span);
        buffer.push(&format!("<impl at {}>", span_str));
    }

    /// Returns the def-id of `def_id`'s parent in the def tree. If
    /// this returns `None`, then `def_id` represents a crate root or
    /// inlined root.
    pub fn parent_def_id(self, def_id: DefId) -> Option<DefId> {
        let key = self.def_key(def_id);
        key.parent.map(|index| DefId { krate: def_id.krate, index: index })
    }
}

/// As a heuristic, when we see an impl, if we see that the
/// 'self-type' is a type defined in the same module as the impl,
/// we can omit including the path to the impl itself. This
/// function tries to find a "characteristic def-id" for a
/// type. It's just a heuristic so it makes some questionable
/// decisions and we may want to adjust it later.
pub fn characteristic_def_id_of_type(ty: Ty) -> Option<DefId> {
    match ty.sty {
        ty::TyAdt(adt_def, _) => Some(adt_def.did),

        ty::TyDynamic(data, ..) => data.principal().map(|p| p.def_id()),

        ty::TyArray(subty, _) |
        ty::TySlice(subty) => characteristic_def_id_of_type(subty),

        ty::TyRawPtr(mt) => characteristic_def_id_of_type(mt.ty),

        ty::TyRef(_, ty, _) => characteristic_def_id_of_type(ty),

        ty::TyTuple(ref tys) => tys.iter()
                                   .filter_map(|ty| characteristic_def_id_of_type(ty))
                                   .next(),

        ty::TyFnDef(def_id, _) |
        ty::TyClosure(def_id, _) |
        ty::TyGenerator(def_id, _, _) |
        ty::TyForeign(def_id) => Some(def_id),

        ty::TyBool |
        ty::TyChar |
        ty::TyInt(_) |
        ty::TyUint(_) |
        ty::TyStr |
        ty::TyFnPtr(_) |
        ty::TyProjection(_) |
        ty::TyParam(_) |
        ty::TyAnon(..) |
        ty::TyInfer(_) |
        ty::TyError |
        ty::TyGeneratorWitness(..) |
        ty::TyNever |
        ty::TyFloat(_) => None,
    }
}

/// Unifying Trait for different kinds of item paths we might
/// construct. The basic interface is that components get pushed: the
/// instance can also customize how we handle the root of a crate.
pub trait ItemPathBuffer {
    fn root_mode(&self) -> &RootMode;
    fn push(&mut self, text: &str);
}

#[derive(Debug)]
pub enum RootMode {
    /// Try to make a path relative to the local crate.  In
    /// particular, local paths have no prefix, and if the path comes
    /// from an extern crate, start with the path to the `extern
    /// crate` declaration.
    Local,

    /// Always prepend the crate name to the path, forming an absolute
    /// path from within a given set of crates.
    Absolute,
}

#[derive(Debug)]
struct LocalPathBuffer {
    root_mode: RootMode,
    str: String,
}

impl LocalPathBuffer {
    fn new(root_mode: RootMode) -> LocalPathBuffer {
        LocalPathBuffer {
            root_mode,
            str: String::new(),
        }
    }

    fn into_string(self) -> String {
        self.str
    }
}

impl ItemPathBuffer for LocalPathBuffer {
    fn root_mode(&self) -> &RootMode {
        &self.root_mode
    }

    fn push(&mut self, text: &str) {
        if !self.str.is_empty() {
            self.str.push_str("::");
        }
        self.str.push_str(text);
    }
}
