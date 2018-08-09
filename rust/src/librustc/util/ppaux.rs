// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use hir::def_id::DefId;
use hir::map::definitions::DefPathData;
use middle::const_val::ConstVal;
use middle::region::{self, BlockRemainder};
use ty::subst::{self, Subst};
use ty::{BrAnon, BrEnv, BrFresh, BrNamed};
use ty::{TyBool, TyChar, TyAdt};
use ty::{TyError, TyStr, TyArray, TySlice, TyFloat, TyFnDef, TyFnPtr};
use ty::{TyParam, TyRawPtr, TyRef, TyNever, TyTuple};
use ty::{TyClosure, TyGenerator, TyGeneratorWitness, TyForeign, TyProjection, TyAnon};
use ty::{TyDynamic, TyInt, TyUint, TyInfer};
use ty::{self, Ty, TyCtxt, TypeFoldable, GenericParamCount, GenericParamDefKind};
use util::nodemap::FxHashSet;

use std::cell::Cell;
use std::fmt;
use std::usize;

use rustc_data_structures::indexed_vec::Idx;
use rustc_target::spec::abi::Abi;
use syntax::ast::CRATE_NODE_ID;
use syntax::symbol::{Symbol, InternedString};
use hir;

macro_rules! gen_display_debug_body {
    ( $with:path ) => {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut cx = PrintContext::new();
            $with(self, f, &mut cx)
        }
    };
}
macro_rules! gen_display_debug {
    ( ($($x:tt)+) $target:ty, display yes ) => {
        impl<$($x)+> fmt::Display for $target {
            gen_display_debug_body! { Print::print_display }
        }
    };
    ( () $target:ty, display yes ) => {
        impl fmt::Display for $target {
            gen_display_debug_body! { Print::print_display }
        }
    };
    ( ($($x:tt)+) $target:ty, debug yes ) => {
        impl<$($x)+> fmt::Debug for $target {
            gen_display_debug_body! { Print::print_debug }
        }
    };
    ( () $target:ty, debug yes ) => {
        impl fmt::Debug for $target {
            gen_display_debug_body! { Print::print_debug }
        }
    };
    ( $generic:tt $target:ty, $t:ident no ) => {};
}
macro_rules! gen_print_impl {
    ( ($($x:tt)+) $target:ty, ($self:ident, $f:ident, $cx:ident) $disp:block $dbg:block ) => {
        impl<$($x)+> Print for $target {
            fn print<F: fmt::Write>(&$self, $f: &mut F, $cx: &mut PrintContext) -> fmt::Result {
                if $cx.is_debug $dbg
                else $disp
            }
        }
    };
    ( () $target:ty, ($self:ident, $f:ident, $cx:ident) $disp:block $dbg:block ) => {
        impl Print for $target {
            fn print<F: fmt::Write>(&$self, $f: &mut F, $cx: &mut PrintContext) -> fmt::Result {
                if $cx.is_debug $dbg
                else $disp
            }
        }
    };
    ( $generic:tt $target:ty,
      $vars:tt $gendisp:ident $disp:block $gendbg:ident $dbg:block ) => {
        gen_print_impl! { $generic $target, $vars $disp $dbg }
        gen_display_debug! { $generic $target, display $gendisp }
        gen_display_debug! { $generic $target, debug $gendbg }
    }
}
macro_rules! define_print {
    ( $generic:tt $target:ty,
      $vars:tt { display $disp:block debug $dbg:block } ) => {
        gen_print_impl! { $generic $target, $vars yes $disp yes $dbg }
    };
    ( $generic:tt $target:ty,
      $vars:tt { debug $dbg:block display $disp:block } ) => {
        gen_print_impl! { $generic $target, $vars yes $disp yes $dbg }
    };
    ( $generic:tt $target:ty,
      $vars:tt { debug $dbg:block } ) => {
        gen_print_impl! { $generic $target, $vars no {
            bug!(concat!("display not implemented for ", stringify!($target)));
        } yes $dbg }
    };
    ( $generic:tt $target:ty,
      ($self:ident, $f:ident, $cx:ident) { display $disp:block } ) => {
        gen_print_impl! { $generic $target, ($self, $f, $cx) yes $disp no {
            write!($f, "{:?}", $self)
        } }
    };
}
macro_rules! define_print_multi {
    ( [ $($generic:tt $target:ty),* ] $vars:tt $def:tt ) => {
        $(define_print! { $generic $target, $vars $def })*
    };
}
macro_rules! print_inner {
    ( $f:expr, $cx:expr, write ($($data:expr),+) ) => {
        write!($f, $($data),+)
    };
    ( $f:expr, $cx:expr, $kind:ident ($data:expr) ) => {
        $data.$kind($f, $cx)
    };
}
macro_rules! print {
    ( $f:expr, $cx:expr $(, $kind:ident $data:tt)+ ) => {
        Ok(())$(.and_then(|_| print_inner!($f, $cx, $kind $data)))+
    };
}


struct LateBoundRegionNameCollector(FxHashSet<InternedString>);
impl<'tcx> ty::fold::TypeVisitor<'tcx> for LateBoundRegionNameCollector {
    fn visit_region(&mut self, r: ty::Region<'tcx>) -> bool {
        match *r {
            ty::ReLateBound(_, ty::BrNamed(_, name)) => {
                self.0.insert(name);
            },
            _ => {},
        }
        r.super_visit_with(self)
    }
}

#[derive(Debug)]
pub struct PrintContext {
    is_debug: bool,
    is_verbose: bool,
    identify_regions: bool,
    used_region_names: Option<FxHashSet<InternedString>>,
    region_index: usize,
    binder_depth: usize,
}
impl PrintContext {
    fn new() -> Self {
        ty::tls::with_opt(|tcx| {
            let (is_verbose, identify_regions) = tcx.map(
                |tcx| (tcx.sess.verbose(), tcx.sess.opts.debugging_opts.identify_regions)
            ).unwrap_or((false, false));
            PrintContext {
                is_debug: false,
                is_verbose: is_verbose,
                identify_regions: identify_regions,
                used_region_names: None,
                region_index: 0,
                binder_depth: 0,
            }
        })
    }
    fn prepare_late_bound_region_info<'tcx, T>(&mut self, value: &ty::Binder<T>)
    where T: TypeFoldable<'tcx>
    {
        let mut collector = LateBoundRegionNameCollector(FxHashSet());
        value.visit_with(&mut collector);
        self.used_region_names = Some(collector.0);
        self.region_index = 0;
    }
}

pub trait Print {
    fn print<F: fmt::Write>(&self, f: &mut F, cx: &mut PrintContext) -> fmt::Result;
    fn print_to_string(&self, cx: &mut PrintContext) -> String {
        let mut result = String::new();
        let _ = self.print(&mut result, cx);
        result
    }
    fn print_display<F: fmt::Write>(&self, f: &mut F, cx: &mut PrintContext) -> fmt::Result {
        let old_debug = cx.is_debug;
        cx.is_debug = false;
        let result = self.print(f, cx);
        cx.is_debug = old_debug;
        result
    }
    fn print_display_to_string(&self, cx: &mut PrintContext) -> String {
        let mut result = String::new();
        let _ = self.print_display(&mut result, cx);
        result
    }
    fn print_debug<F: fmt::Write>(&self, f: &mut F, cx: &mut PrintContext) -> fmt::Result {
        let old_debug = cx.is_debug;
        cx.is_debug = true;
        let result = self.print(f, cx);
        cx.is_debug = old_debug;
        result
    }
    fn print_debug_to_string(&self, cx: &mut PrintContext) -> String {
        let mut result = String::new();
        let _ = self.print_debug(&mut result, cx);
        result
    }
}

impl PrintContext {
    fn fn_sig<F: fmt::Write>(&mut self,
                             f: &mut F,
                             inputs: &[Ty],
                             variadic: bool,
                             output: Ty)
                             -> fmt::Result {
        write!(f, "(")?;
        let mut inputs = inputs.iter();
        if let Some(&ty) = inputs.next() {
            print!(f, self, print_display(ty))?;
            for &ty in inputs {
                print!(f, self, write(", "), print_display(ty))?;
            }
            if variadic {
                write!(f, ", ...")?;
            }
        }
        write!(f, ")")?;
        if !output.is_nil() {
            print!(f, self, write(" -> "), print_display(output))?;
        }

        Ok(())
    }

    fn parameterized<F: fmt::Write>(&mut self,
                                    f: &mut F,
                                    substs: &subst::Substs,
                                    mut did: DefId,
                                    projections: &[ty::ProjectionPredicate])
                                    -> fmt::Result {
        let key = ty::tls::with(|tcx| tcx.def_key(did));
        let mut item_name = if let Some(name) = key.disambiguated_data.data.get_opt_name() {
            Some(name)
        } else {
            did.index = key.parent.unwrap_or_else(
                || bug!("finding type for {:?}, encountered def-id {:?} with no parent",
                        did, did));
            self.parameterized(f, substs, did, projections)?;
            return write!(f, "::{}", key.disambiguated_data.data.as_interned_str());
        };

        let verbose = self.is_verbose;
        let mut num_supplied_defaults = 0;
        let mut has_self = false;
        let mut own_counts = GenericParamCount {
            lifetimes: 0,
            types: 0,
        };
        let mut is_value_path = false;
        let fn_trait_kind = ty::tls::with(|tcx| {
            // Unfortunately, some kinds of items (e.g., closures) don't have
            // generics. So walk back up the find the closest parent that DOES
            // have them.
            let mut item_def_id = did;
            loop {
                let key = tcx.def_key(item_def_id);
                match key.disambiguated_data.data {
                    DefPathData::AssocTypeInTrait(_) |
                    DefPathData::AssocTypeInImpl(_) |
                    DefPathData::Trait(_) |
                    DefPathData::TypeNs(_) => {
                        break;
                    }
                    DefPathData::ValueNs(_) |
                    DefPathData::EnumVariant(_) => {
                        is_value_path = true;
                        break;
                    }
                    DefPathData::CrateRoot |
                    DefPathData::Misc |
                    DefPathData::Impl |
                    DefPathData::Module(_) |
                    DefPathData::MacroDef(_) |
                    DefPathData::ClosureExpr |
                    DefPathData::TypeParam(_) |
                    DefPathData::LifetimeDef(_) |
                    DefPathData::Field(_) |
                    DefPathData::StructCtor |
                    DefPathData::AnonConst |
                    DefPathData::ExistentialImplTrait |
                    DefPathData::UniversalImplTrait |
                    DefPathData::GlobalMetaData(_) => {
                        // if we're making a symbol for something, there ought
                        // to be a value or type-def or something in there
                        // *somewhere*
                        item_def_id.index = key.parent.unwrap_or_else(|| {
                            bug!("finding type for {:?}, encountered def-id {:?} with no \
                                 parent", did, item_def_id);
                        });
                    }
                }
            }
            let mut generics = tcx.generics_of(item_def_id);
            let child_own_counts = generics.own_counts();
            let mut path_def_id = did;
            has_self = generics.has_self;

            let mut child_types = 0;
            if let Some(def_id) = generics.parent {
                // Methods.
                assert!(is_value_path);
                child_types = child_own_counts.types;
                generics = tcx.generics_of(def_id);
                own_counts = generics.own_counts();

                if has_self {
                    print!(f, self, write("<"), print_display(substs.type_at(0)), write(" as "))?;
                }

                path_def_id = def_id;
            } else {
                item_name = None;

                if is_value_path {
                    // Functions.
                    assert_eq!(has_self, false);
                } else {
                    // Types and traits.
                    own_counts = child_own_counts;
                }
            }

            if !verbose {
                let mut type_params =
                    generics.params.iter().rev().filter_map(|param| {
                        match param.kind {
                            GenericParamDefKind::Type { has_default, .. } => {
                                Some((param.def_id, has_default))
                            }
                            GenericParamDefKind::Lifetime => None,
                        }
                    }).peekable();
                let has_default = {
                    let has_default = type_params.peek().map(|(_, has_default)| has_default);
                    *has_default.unwrap_or(&false)
                };
                if has_default {
                    if let Some(substs) = tcx.lift(&substs) {
                        let mut types = substs.types().rev().skip(child_types);
                        for ((def_id, has_default), actual) in type_params.zip(types) {
                            if !has_default {
                                break;
                            }
                            if tcx.type_of(def_id).subst(tcx, substs) != actual {
                                break;
                            }
                            num_supplied_defaults += 1;
                        }
                    }
                }
            }

            print!(f, self, write("{}", tcx.item_path_str(path_def_id)))?;
            Ok(tcx.lang_items().fn_trait_kind(path_def_id))
        })?;

        if !verbose && fn_trait_kind.is_some() && projections.len() == 1 {
            let projection_ty = projections[0].ty;
            if let TyTuple(ref args) = substs.type_at(1).sty {
                return self.fn_sig(f, args, false, projection_ty);
            }
        }

        let empty = Cell::new(true);
        let start_or_continue = |f: &mut F, start: &str, cont: &str| {
            if empty.get() {
                empty.set(false);
                write!(f, "{}", start)
            } else {
                write!(f, "{}", cont)
            }
        };

        let print_regions = |f: &mut F, start: &str, skip, count| {
            // Don't print any regions if they're all erased.
            let regions = || substs.regions().skip(skip).take(count);
            if regions().all(|r: ty::Region| *r == ty::ReErased) {
                return Ok(());
            }

            for region in regions() {
                let region: ty::Region = region;
                start_or_continue(f, start, ", ")?;
                if verbose {
                    write!(f, "{:?}", region)?;
                } else {
                    let s = region.to_string();
                    if s.is_empty() {
                        // This happens when the value of the region
                        // parameter is not easily serialized. This may be
                        // because the user omitted it in the first place,
                        // or because it refers to some block in the code,
                        // etc. I'm not sure how best to serialize this.
                        write!(f, "'_")?;
                    } else {
                        write!(f, "{}", s)?;
                    }
                }
            }

            Ok(())
        };

        print_regions(f, "<", 0, own_counts.lifetimes)?;

        let tps = substs.types()
                        .take(own_counts.types - num_supplied_defaults)
                        .skip(has_self as usize);

        for ty in tps {
            start_or_continue(f, "<", ", ")?;
            ty.print_display(f, self)?;
        }

        for projection in projections {
            start_or_continue(f, "<", ", ")?;
            ty::tls::with(|tcx|
                print!(f, self,
                       write("{}=",
                             tcx.associated_item(projection.projection_ty.item_def_id).name),
                       print_display(projection.ty))
            )?;
        }

        start_or_continue(f, "", ">")?;

        // For values, also print their name and type parameters.
        if is_value_path {
            empty.set(true);

            if has_self {
                write!(f, ">")?;
            }

            if let Some(item_name) = item_name {
                write!(f, "::{}", item_name)?;
            }

            print_regions(f, "::<", own_counts.lifetimes, usize::MAX)?;

            // FIXME: consider being smart with defaults here too
            for ty in substs.types().skip(own_counts.types) {
                start_or_continue(f, "::<", ", ")?;
                ty.print_display(f, self)?;
            }

            start_or_continue(f, "", ">")?;
        }

        Ok(())
    }

    fn in_binder<'a, 'gcx, 'tcx, T, U, F>(&mut self,
                                          f: &mut F,
                                          tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                          original: &ty::Binder<T>,
                                          lifted: Option<ty::Binder<U>>) -> fmt::Result
        where T: Print, U: Print + TypeFoldable<'tcx>, F: fmt::Write
    {
        fn name_by_region_index(index: usize) -> InternedString {
            match index {
                0 => Symbol::intern("'r"),
                1 => Symbol::intern("'s"),
                i => Symbol::intern(&format!("'t{}", i-2)),
            }.as_interned_str()
        }

        // Replace any anonymous late-bound regions with named
        // variants, using gensym'd identifiers, so that we can
        // clearly differentiate between named and unnamed regions in
        // the output. We'll probably want to tweak this over time to
        // decide just how much information to give.
        let value = if let Some(v) = lifted {
            v
        } else {
            return original.skip_binder().print_display(f, self);
        };

        if self.binder_depth == 0 {
            self.prepare_late_bound_region_info(&value);
        }

        let mut empty = true;
        let mut start_or_continue = |f: &mut F, start: &str, cont: &str| {
            if empty {
                empty = false;
                write!(f, "{}", start)
            } else {
                write!(f, "{}", cont)
            }
        };

        let old_region_index = self.region_index;
        let mut region_index = old_region_index;
        let new_value = tcx.replace_late_bound_regions(&value, |br| {
            let _ = start_or_continue(f, "for<", ", ");
            let br = match br {
                ty::BrNamed(_, name) => {
                    let _ = write!(f, "{}", name);
                    br
                }
                ty::BrAnon(_) |
                ty::BrFresh(_) |
                ty::BrEnv => {
                    let name = loop {
                        let name = name_by_region_index(region_index);
                        region_index += 1;
                        if !self.is_name_used(&name) {
                            break name;
                        }
                    };
                    let _ = write!(f, "{}", name);
                    ty::BrNamed(tcx.hir.local_def_id(CRATE_NODE_ID), name)
                }
            };
            tcx.mk_region(ty::ReLateBound(ty::INNERMOST, br))
        }).0;
        start_or_continue(f, "", "> ")?;

        // Push current state to gcx, and restore after writing new_value.
        self.binder_depth += 1;
        self.region_index = region_index;
        let result = new_value.print_display(f, self);
        self.region_index = old_region_index;
        self.binder_depth -= 1;
        result
    }

    fn is_name_used(&self, name: &InternedString) -> bool {
        match self.used_region_names {
            Some(ref names) => names.contains(name),
            None => false,
        }
    }
}

pub fn verbose() -> bool {
    ty::tls::with(|tcx| tcx.sess.verbose())
}

pub fn identify_regions() -> bool {
    ty::tls::with(|tcx| tcx.sess.opts.debugging_opts.identify_regions)
}

pub fn parameterized<F: fmt::Write>(f: &mut F,
                                    substs: &subst::Substs,
                                    did: DefId,
                                    projections: &[ty::ProjectionPredicate])
                                    -> fmt::Result {
    PrintContext::new().parameterized(f, substs, did, projections)
}


impl<'a, T: Print> Print for &'a T {
    fn print<F: fmt::Write>(&self, f: &mut F, cx: &mut PrintContext) -> fmt::Result {
        (*self).print(f, cx)
    }
}

define_print! {
    ('tcx) &'tcx ty::Slice<ty::ExistentialPredicate<'tcx>>, (self, f, cx) {
        display {
            // Generate the main trait ref, including associated types.
            ty::tls::with(|tcx| {
                // Use a type that can't appear in defaults of type parameters.
                let dummy_self = tcx.mk_infer(ty::FreshTy(0));

                if let Some(p) = self.principal() {
                    let principal = tcx.lift(&p).expect("could not lift TraitRef for printing")
                        .with_self_ty(tcx, dummy_self);
                    let projections = self.projection_bounds().map(|p| {
                        tcx.lift(&p)
                            .expect("could not lift projection for printing")
                            .with_self_ty(tcx, dummy_self)
                    }).collect::<Vec<_>>();
                    cx.parameterized(f, principal.substs, principal.def_id, &projections)?;
                }

                // Builtin bounds.
                for did in self.auto_traits() {
                    write!(f, " + {}", tcx.item_path_str(did))?;
                }

                Ok(())
            })?;

            Ok(())
        }
    }
}

impl fmt::Debug for ty::GenericParamDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let type_name = match self.kind {
            ty::GenericParamDefKind::Lifetime => "Lifetime",
            ty::GenericParamDefKind::Type {..} => "Type",
        };
        write!(f, "{}({}, {:?}, {})",
               type_name,
               self.name,
               self.def_id,
               self.index)
    }
}

impl fmt::Debug for ty::TraitDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ty::tls::with(|tcx| {
            write!(f, "{}", tcx.item_path_str(self.def_id))
        })
    }
}

impl fmt::Debug for ty::AdtDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ty::tls::with(|tcx| {
            write!(f, "{}", tcx.item_path_str(self.did))
        })
    }
}

impl<'tcx> fmt::Debug for ty::ClosureUpvar<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ClosureUpvar({:?},{:?})",
               self.def,
               self.ty)
    }
}

impl fmt::Debug for ty::UpvarId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UpvarId({:?};`{}`;{:?})",
               self.var_id,
               ty::tls::with(|tcx| tcx.hir.name(tcx.hir.hir_to_node_id(self.var_id))),
               self.closure_expr_id)
    }
}

impl<'tcx> fmt::Debug for ty::UpvarBorrow<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UpvarBorrow({:?}, {:?})",
               self.kind, self.region)
    }
}

define_print! {
    ('tcx) &'tcx ty::Slice<Ty<'tcx>>, (self, f, cx) {
        display {
            write!(f, "{{")?;
            let mut tys = self.iter();
            if let Some(&ty) = tys.next() {
                print!(f, cx, print(ty))?;
                for &ty in tys {
                    print!(f, cx, write(", "), print(ty))?;
                }
            }
            write!(f, "}}")
        }
    }
}

define_print! {
    ('tcx) ty::TypeAndMut<'tcx>, (self, f, cx) {
        display {
            print!(f, cx,
                   write("{}", if self.mutbl == hir::MutMutable { "mut " } else { "" }),
                   print(self.ty))
        }
    }
}

define_print! {
    ('tcx) ty::ExistentialTraitRef<'tcx>, (self, f, cx) {
        debug {
            ty::tls::with(|tcx| {
                let dummy_self = tcx.mk_infer(ty::FreshTy(0));

                let trait_ref = *tcx.lift(&ty::Binder::bind(*self))
                                   .expect("could not lift TraitRef for printing")
                                   .with_self_ty(tcx, dummy_self).skip_binder();
                cx.parameterized(f, trait_ref.substs, trait_ref.def_id, &[])
            })
        }
    }
}

define_print! {
    ('tcx) ty::adjustment::Adjustment<'tcx>, (self, f, cx) {
        debug {
            print!(f, cx, write("{:?} -> ", self.kind), print(self.target))
        }
    }
}

define_print! {
    () ty::BoundRegion, (self, f, cx) {
        display {
            if cx.is_verbose {
                return self.print_debug(f, cx);
            }

            match *self {
                BrNamed(_, name) => write!(f, "{}", name),
                BrAnon(_) | BrFresh(_) | BrEnv => Ok(())
            }
        }
        debug {
            return match *self {
                BrAnon(n) => write!(f, "BrAnon({:?})", n),
                BrFresh(n) => write!(f, "BrFresh({:?})", n),
                BrNamed(did, name) => {
                    write!(f, "BrNamed({:?}:{:?}, {})",
                           did.krate, did.index, name)
                }
                BrEnv => write!(f, "BrEnv"),
            };
        }
    }
}

define_print! {
    () ty::RegionKind, (self, f, cx) {
        display {
            if cx.is_verbose {
                return self.print_debug(f, cx);
            }

            // These printouts are concise.  They do not contain all the information
            // the user might want to diagnose an error, but there is basically no way
            // to fit that into a short string.  Hence the recommendation to use
            // `explain_region()` or `note_and_explain_region()`.
            match *self {
                ty::ReEarlyBound(ref data) => {
                    write!(f, "{}", data.name)
                }
                ty::ReCanonical(_) => {
                    write!(f, "'_")
                }
                ty::ReLateBound(_, br) |
                ty::ReFree(ty::FreeRegion { bound_region: br, .. }) |
                ty::ReSkolemized(_, br) => {
                    write!(f, "{}", br)
                }
                ty::ReScope(scope) if cx.identify_regions => {
                    match scope.data() {
                        region::ScopeData::Node(id) =>
                            write!(f, "'{}s", id.as_usize()),
                        region::ScopeData::CallSite(id) =>
                            write!(f, "'{}cs", id.as_usize()),
                        region::ScopeData::Arguments(id) =>
                            write!(f, "'{}as", id.as_usize()),
                        region::ScopeData::Destruction(id) =>
                            write!(f, "'{}ds", id.as_usize()),
                        region::ScopeData::Remainder(BlockRemainder
                                                     { block, first_statement_index }) =>
                            write!(f, "'{}_{}rs", block.as_usize(), first_statement_index.index()),
                    }
                }
                ty::ReVar(region_vid) if cx.identify_regions => {
                    write!(f, "'{}rv", region_vid.index())
                }
                ty::ReScope(_) |
                ty::ReVar(_) |
                ty::ReErased => Ok(()),
                ty::ReStatic => write!(f, "'static"),
                ty::ReEmpty => write!(f, "'<empty>"),

                // The user should never encounter these in unsubstituted form.
                ty::ReClosureBound(vid) => write!(f, "{:?}", vid),
            }
        }
        debug {
            match *self {
                ty::ReEarlyBound(ref data) => {
                    write!(f, "ReEarlyBound({}, {})",
                           data.index,
                           data.name)
                }

                ty::ReClosureBound(ref vid) => {
                    write!(f, "ReClosureBound({:?})",
                           vid)
                }

                ty::ReLateBound(binder_id, ref bound_region) => {
                    write!(f, "ReLateBound({:?}, {:?})",
                           binder_id,
                           bound_region)
                }

                ty::ReFree(ref fr) => write!(f, "{:?}", fr),

                ty::ReScope(id) => {
                    write!(f, "ReScope({:?})", id)
                }

                ty::ReStatic => write!(f, "ReStatic"),

                ty::ReVar(ref vid) => {
                    write!(f, "{:?}", vid)
                }

                ty::ReCanonical(c) => {
                    write!(f, "'?{}", c.index())
                }

                ty::ReSkolemized(universe, ref bound_region) => {
                    write!(f, "ReSkolemized({:?}, {:?})", universe, bound_region)
                }

                ty::ReEmpty => write!(f, "ReEmpty"),

                ty::ReErased => write!(f, "ReErased")
            }
        }
    }
}

define_print! {
    () ty::FreeRegion, (self, f, cx) {
        debug {
            write!(f, "ReFree({:?}, {:?})", self.scope, self.bound_region)
        }
    }
}

define_print! {
    () ty::Variance, (self, f, cx) {
        debug {
            f.write_str(match *self {
                ty::Covariant => "+",
                ty::Contravariant => "-",
                ty::Invariant => "o",
                ty::Bivariant => "*",
            })
        }
    }
}

define_print! {
    ('tcx) ty::GenericPredicates<'tcx>, (self, f, cx) {
        debug {
            write!(f, "GenericPredicates({:?})", self.predicates)
        }
    }
}

define_print! {
    ('tcx) ty::InstantiatedPredicates<'tcx>, (self, f, cx) {
        debug {
            write!(f, "InstantiatedPredicates({:?})", self.predicates)
        }
    }
}

define_print! {
    ('tcx) ty::FnSig<'tcx>, (self, f, cx) {
        display {
            if self.unsafety == hir::Unsafety::Unsafe {
                write!(f, "unsafe ")?;
            }

            if self.abi != Abi::Rust {
                write!(f, "extern {} ", self.abi)?;
            }

            write!(f, "fn")?;
            cx.fn_sig(f, self.inputs(), self.variadic, self.output())
        }
        debug {
            write!(f, "({:?}; variadic: {})->{:?}", self.inputs(), self.variadic, self.output())
        }
    }
}

impl fmt::Debug for ty::TyVid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "_#{}t", self.index)
    }
}

impl fmt::Debug for ty::IntVid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "_#{}i", self.index)
    }
}

impl fmt::Debug for ty::FloatVid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "_#{}f", self.index)
    }
}

impl fmt::Debug for ty::RegionVid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'_#{}r", self.index())
    }
}

define_print! {
    () ty::InferTy, (self, f, cx) {
        display {
            if cx.is_verbose {
                print!(f, cx, print_debug(self))
            } else {
                match *self {
                    ty::TyVar(_) => write!(f, "_"),
                    ty::IntVar(_) => write!(f, "{}", "{integer}"),
                    ty::FloatVar(_) => write!(f, "{}", "{float}"),
                    ty::CanonicalTy(_) => write!(f, "_"),
                    ty::FreshTy(v) => write!(f, "FreshTy({})", v),
                    ty::FreshIntTy(v) => write!(f, "FreshIntTy({})", v),
                    ty::FreshFloatTy(v) => write!(f, "FreshFloatTy({})", v)
                }
            }
        }
        debug {
            match *self {
                ty::TyVar(ref v) => write!(f, "{:?}", v),
                ty::IntVar(ref v) => write!(f, "{:?}", v),
                ty::FloatVar(ref v) => write!(f, "{:?}", v),
                ty::CanonicalTy(v) => write!(f, "?{:?}", v.index()),
                ty::FreshTy(v) => write!(f, "FreshTy({:?})", v),
                ty::FreshIntTy(v) => write!(f, "FreshIntTy({:?})", v),
                ty::FreshFloatTy(v) => write!(f, "FreshFloatTy({:?})", v)
            }
        }
    }
}

impl fmt::Debug for ty::IntVarValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ty::IntType(ref v) => v.fmt(f),
            ty::UintType(ref v) => v.fmt(f),
        }
    }
}

impl fmt::Debug for ty::FloatVarValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

// The generic impl doesn't work yet because projections are not
// normalized under HRTB.
/*impl<T> fmt::Display for ty::Binder<T>
    where T: fmt::Display + for<'a> ty::Lift<'a>,
          for<'a> <T as ty::Lift<'a>>::Lifted: fmt::Display + TypeFoldable<'a>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ty::tls::with(|tcx| in_binder(f, tcx, self, tcx.lift(self)))
    }
}*/

define_print_multi! {
    [
    ('tcx) ty::Binder<&'tcx ty::Slice<ty::ExistentialPredicate<'tcx>>>,
    ('tcx) ty::Binder<ty::TraitRef<'tcx>>,
    ('tcx) ty::Binder<ty::FnSig<'tcx>>,
    ('tcx) ty::Binder<ty::TraitPredicate<'tcx>>,
    ('tcx) ty::Binder<ty::SubtypePredicate<'tcx>>,
    ('tcx) ty::Binder<ty::ProjectionPredicate<'tcx>>,
    ('tcx) ty::Binder<ty::OutlivesPredicate<Ty<'tcx>, ty::Region<'tcx>>>,
    ('tcx) ty::Binder<ty::OutlivesPredicate<ty::Region<'tcx>, ty::Region<'tcx>>>
    ]
    (self, f, cx) {
        display {
            ty::tls::with(|tcx| cx.in_binder(f, tcx, self, tcx.lift(self)))
        }
    }
}

define_print! {
    ('tcx) ty::TraitRef<'tcx>, (self, f, cx) {
        display {
            cx.parameterized(f, self.substs, self.def_id, &[])
        }
        debug {
            // when printing out the debug representation, we don't need
            // to enumerate the `for<...>` etc because the debruijn index
            // tells you everything you need to know.
            print!(f, cx,
                   write("<"),
                   print(self.self_ty()),
                   write(" as "))?;
            cx.parameterized(f, self.substs, self.def_id, &[])?;
            write!(f, ">")
        }
    }
}

define_print! {
    ('tcx) ty::TypeVariants<'tcx>, (self, f, cx) {
        display {
            match *self {
                TyBool => write!(f, "bool"),
                TyChar => write!(f, "char"),
                TyInt(t) => write!(f, "{}", t.ty_to_string()),
                TyUint(t) => write!(f, "{}", t.ty_to_string()),
                TyFloat(t) => write!(f, "{}", t.ty_to_string()),
                TyRawPtr(ref tm) => {
                    write!(f, "*{} ", match tm.mutbl {
                        hir::MutMutable => "mut",
                        hir::MutImmutable => "const",
                    })?;
                    tm.ty.print(f, cx)
                }
                TyRef(r, ty, mutbl) => {
                    write!(f, "&")?;
                    let s = r.print_to_string(cx);
                    write!(f, "{}", s)?;
                    if !s.is_empty() {
                        write!(f, " ")?;
                    }
                    ty::TypeAndMut { ty, mutbl }.print(f, cx)
                }
                TyNever => write!(f, "!"),
                TyTuple(ref tys) => {
                    write!(f, "(")?;
                    let mut tys = tys.iter();
                    if let Some(&ty) = tys.next() {
                        print!(f, cx, print(ty), write(","))?;
                        if let Some(&ty) = tys.next() {
                            print!(f, cx, write(" "), print(ty))?;
                            for &ty in tys {
                                print!(f, cx, write(", "), print(ty))?;
                            }
                        }
                    }
                    write!(f, ")")
                }
                TyFnDef(def_id, substs) => {
                    ty::tls::with(|tcx| {
                        let mut sig = tcx.fn_sig(def_id);
                        if let Some(substs) = tcx.lift(&substs) {
                            sig = sig.subst(tcx, substs);
                        }
                        print!(f, cx, print(sig), write(" {{"))
                    })?;
                    cx.parameterized(f, substs, def_id, &[])?;
                    write!(f, "}}")
                }
                TyFnPtr(ref bare_fn) => {
                    bare_fn.print(f, cx)
                }
                TyInfer(infer_ty) => write!(f, "{}", infer_ty),
                TyError => write!(f, "[type error]"),
                TyParam(ref param_ty) => write!(f, "{}", param_ty),
                TyAdt(def, substs) => cx.parameterized(f, substs, def.did, &[]),
                TyDynamic(data, r) => {
                    data.print(f, cx)?;
                    let r = r.print_to_string(cx);
                    if !r.is_empty() {
                        write!(f, " + {}", r)
                    } else {
                        Ok(())
                    }
                }
                TyForeign(def_id) => parameterized(f, subst::Substs::empty(), def_id, &[]),
                TyProjection(ref data) => data.print(f, cx),
                TyAnon(def_id, substs) => {
                    if cx.is_verbose {
                        return write!(f, "TyAnon({:?}, {:?})", def_id, substs);
                    }

                    ty::tls::with(|tcx| {
                        // Grab the "TraitA + TraitB" from `impl TraitA + TraitB`,
                        // by looking up the projections associated with the def_id.
                        let predicates_of = tcx.predicates_of(def_id);
                        let substs = tcx.lift(&substs).unwrap_or_else(|| {
                            tcx.intern_substs(&[])
                        });
                        let bounds = predicates_of.instantiate(tcx, substs);

                        let mut first = true;
                        let mut is_sized = false;
                        write!(f, "impl")?;
                        for predicate in bounds.predicates {
                            if let Some(trait_ref) = predicate.to_opt_poly_trait_ref() {
                                // Don't print +Sized, but rather +?Sized if absent.
                                if Some(trait_ref.def_id()) == tcx.lang_items().sized_trait() {
                                    is_sized = true;
                                    continue;
                                }

                                print!(f, cx,
                                       write("{}", if first { " " } else { "+" }),
                                       print(trait_ref))?;
                                first = false;
                            }
                        }
                        if !is_sized {
                            write!(f, "{}?Sized", if first { " " } else { "+" })?;
                        }
                        Ok(())
                    })
                }
                TyStr => write!(f, "str"),
                TyGenerator(did, substs, movability) => ty::tls::with(|tcx| {
                    let upvar_tys = substs.upvar_tys(did, tcx);
                    let witness = substs.witness(did, tcx);
                    if movability == hir::GeneratorMovability::Movable {
                        write!(f, "[generator")?;
                    } else {
                        write!(f, "[static generator")?;
                    }

                    if let Some(node_id) = tcx.hir.as_local_node_id(did) {
                        write!(f, "@{:?}", tcx.hir.span(node_id))?;
                        let mut sep = " ";
                        tcx.with_freevars(node_id, |freevars| {
                            for (freevar, upvar_ty) in freevars.iter().zip(upvar_tys) {
                                print!(f, cx,
                                       write("{}{}:",
                                             sep,
                                             tcx.hir.name(freevar.var_id())),
                                       print(upvar_ty))?;
                                sep = ", ";
                            }
                            Ok(())
                        })?
                    } else {
                        // cross-crate closure types should only be
                        // visible in codegen bug reports, I imagine.
                        write!(f, "@{:?}", did)?;
                        let mut sep = " ";
                        for (index, upvar_ty) in upvar_tys.enumerate() {
                            print!(f, cx,
                                   write("{}{}:", sep, index),
                                   print(upvar_ty))?;
                            sep = ", ";
                        }
                    }

                    print!(f, cx, write(" "), print(witness), write("]"))
                }),
                TyGeneratorWitness(types) => {
                    ty::tls::with(|tcx| cx.in_binder(f, tcx, &types, tcx.lift(&types)))
                }
                TyClosure(did, substs) => ty::tls::with(|tcx| {
                    let upvar_tys = substs.upvar_tys(did, tcx);
                    write!(f, "[closure")?;

                    if let Some(node_id) = tcx.hir.as_local_node_id(did) {
                        if tcx.sess.opts.debugging_opts.span_free_formats {
                            write!(f, "@{:?}", node_id)?;
                        } else {
                            write!(f, "@{:?}", tcx.hir.span(node_id))?;
                        }
                        let mut sep = " ";
                        tcx.with_freevars(node_id, |freevars| {
                            for (freevar, upvar_ty) in freevars.iter().zip(upvar_tys) {
                                print!(f, cx,
                                       write("{}{}:",
                                             sep,
                                             tcx.hir.name(freevar.var_id())),
                                       print(upvar_ty))?;
                                sep = ", ";
                            }
                            Ok(())
                        })?
                    } else {
                        // cross-crate closure types should only be
                        // visible in codegen bug reports, I imagine.
                        write!(f, "@{:?}", did)?;
                        let mut sep = " ";
                        for (index, upvar_ty) in upvar_tys.enumerate() {
                            print!(f, cx,
                                   write("{}{}:", sep, index),
                                   print(upvar_ty))?;
                            sep = ", ";
                        }
                    }

                    write!(f, "]")
                }),
                TyArray(ty, sz) => {
                    print!(f, cx, write("["), print(ty), write("; "))?;
                    match sz.val {
                        ConstVal::Value(..) => ty::tls::with(|tcx| {
                            write!(f, "{}", sz.unwrap_usize(tcx))
                        })?,
                        ConstVal::Unevaluated(_def_id, _substs) => {
                            write!(f, "_")?;
                        }
                    }
                    write!(f, "]")
                }
                TySlice(ty) => {
                    print!(f, cx, write("["), print(ty), write("]"))
                }
            }
        }
    }
}

define_print! {
    ('tcx) ty::TyS<'tcx>, (self, f, cx) {
        display {
            self.sty.print(f, cx)
        }
        debug {
            self.sty.print_display(f, cx)
        }
    }
}

define_print! {
    () ty::ParamTy, (self, f, cx) {
        display {
            write!(f, "{}", self.name)
        }
        debug {
            write!(f, "{}/#{}", self.name, self.idx)
        }
    }
}

define_print! {
    ('tcx, T: Print + fmt::Debug, U: Print + fmt::Debug) ty::OutlivesPredicate<T, U>,
    (self, f, cx) {
        display {
            print!(f, cx, print(self.0), write(" : "), print(self.1))
        }
    }
}

define_print! {
    ('tcx) ty::SubtypePredicate<'tcx>, (self, f, cx) {
        display {
            print!(f, cx, print(self.a), write(" <: "), print(self.b))
        }
    }
}

define_print! {
    ('tcx) ty::TraitPredicate<'tcx>, (self, f, cx) {
        debug {
            write!(f, "TraitPredicate({:?})",
                   self.trait_ref)
        }
        display {
            print!(f, cx, print(self.trait_ref.self_ty()), write(": "), print(self.trait_ref))
        }
    }
}

define_print! {
    ('tcx) ty::ProjectionPredicate<'tcx>, (self, f, cx) {
        debug {
            print!(f, cx,
                   write("ProjectionPredicate("),
                   print(self.projection_ty),
                   write(", "),
                   print(self.ty),
                   write(")"))
        }
        display {
            print!(f, cx, print(self.projection_ty), write(" == "), print(self.ty))
        }
    }
}

define_print! {
    ('tcx) ty::ProjectionTy<'tcx>, (self, f, cx) {
        display {
            // FIXME(tschottdorf): use something like
            //   parameterized(f, self.substs, self.item_def_id, &[])
            // (which currently ICEs).
            let (trait_ref, item_name) = ty::tls::with(|tcx|
                (self.trait_ref(tcx), tcx.associated_item(self.item_def_id).name)
            );
            print!(f, cx, print_debug(trait_ref), write("::{}", item_name))
        }
    }
}

define_print! {
    () ty::ClosureKind, (self, f, cx) {
        display {
            match *self {
                ty::ClosureKind::Fn => write!(f, "Fn"),
                ty::ClosureKind::FnMut => write!(f, "FnMut"),
                ty::ClosureKind::FnOnce => write!(f, "FnOnce"),
            }
        }
    }
}

define_print! {
    ('tcx) ty::Predicate<'tcx>, (self, f, cx) {
        display {
            match *self {
                ty::Predicate::Trait(ref data) => data.print(f, cx),
                ty::Predicate::Subtype(ref predicate) => predicate.print(f, cx),
                ty::Predicate::RegionOutlives(ref predicate) => predicate.print(f, cx),
                ty::Predicate::TypeOutlives(ref predicate) => predicate.print(f, cx),
                ty::Predicate::Projection(ref predicate) => predicate.print(f, cx),
                ty::Predicate::WellFormed(ty) => print!(f, cx, print(ty), write(" well-formed")),
                ty::Predicate::ObjectSafe(trait_def_id) =>
                    ty::tls::with(|tcx| {
                        write!(f, "the trait `{}` is object-safe", tcx.item_path_str(trait_def_id))
                    }),
                ty::Predicate::ClosureKind(closure_def_id, _closure_substs, kind) =>
                    ty::tls::with(|tcx| {
                        write!(f, "the closure `{}` implements the trait `{}`",
                               tcx.item_path_str(closure_def_id), kind)
                    }),
                ty::Predicate::ConstEvaluatable(def_id, substs) => {
                    write!(f, "the constant `")?;
                    cx.parameterized(f, substs, def_id, &[])?;
                    write!(f, "` can be evaluated")
                }
            }
        }
        debug {
            match *self {
                ty::Predicate::Trait(ref a) => a.print(f, cx),
                ty::Predicate::Subtype(ref pair) => pair.print(f, cx),
                ty::Predicate::RegionOutlives(ref pair) => pair.print(f, cx),
                ty::Predicate::TypeOutlives(ref pair) => pair.print(f, cx),
                ty::Predicate::Projection(ref pair) => pair.print(f, cx),
                ty::Predicate::WellFormed(ty) => ty.print(f, cx),
                ty::Predicate::ObjectSafe(trait_def_id) => {
                    write!(f, "ObjectSafe({:?})", trait_def_id)
                }
                ty::Predicate::ClosureKind(closure_def_id, closure_substs, kind) => {
                    write!(f, "ClosureKind({:?}, {:?}, {:?})", closure_def_id, closure_substs, kind)
                }
                ty::Predicate::ConstEvaluatable(def_id, substs) => {
                    write!(f, "ConstEvaluatable({:?}, {:?})", def_id, substs)
                }
            }
        }
    }
}
