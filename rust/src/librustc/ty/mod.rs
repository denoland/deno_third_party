// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub use self::Variance::*;
pub use self::AssociatedItemContainer::*;
pub use self::BorrowKind::*;
pub use self::IntVarValue::*;
pub use self::fold::TypeFoldable;

use hir::{map as hir_map, FreevarMap, TraitMap};
use hir::def::{Def, CtorKind, ExportMap};
use hir::def_id::{CrateNum, DefId, LocalDefId, CRATE_DEF_INDEX, LOCAL_CRATE};
use hir::map::DefPathData;
use hir::svh::Svh;
use ich::Fingerprint;
use ich::StableHashingContext;
use infer::canonical::{Canonical, Canonicalize};
use middle::lang_items::{FnTraitLangItem, FnMutTraitLangItem, FnOnceTraitLangItem};
use middle::privacy::AccessLevels;
use middle::resolve_lifetime::ObjectLifetimeDefault;
use mir::Mir;
use mir::interpret::GlobalId;
use mir::GeneratorLayout;
use session::CrateDisambiguator;
use traits::{self, Reveal};
use ty;
use ty::subst::{Subst, Substs};
use ty::util::{IntTypeExt, Discr};
use ty::walk::TypeWalker;
use util::captures::Captures;
use util::nodemap::{NodeSet, DefIdMap, FxHashMap};
use arena::SyncDroplessArena;

use serialize::{self, Encodable, Encoder};
use std::cell::RefCell;
use std::cmp::{self, Ordering};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use rustc_data_structures::sync::{self, Lrc, ParallelIterator, par_iter};
use std::slice;
use std::vec::IntoIter;
use std::mem;
use syntax::ast::{self, DUMMY_NODE_ID, Name, Ident, NodeId};
use syntax::attr;
use syntax::ext::hygiene::Mark;
use syntax::symbol::{Symbol, LocalInternedString, InternedString};
use syntax_pos::{DUMMY_SP, Span};

use rustc_data_structures::accumulate_vec::IntoIter as AccIntoIter;
use rustc_data_structures::stable_hasher::{StableHasher, StableHasherResult,
                                           HashStable};

use hir;

pub use self::sty::{Binder, CanonicalVar, DebruijnIndex, INNERMOST};
pub use self::sty::{FnSig, GenSig, PolyFnSig, PolyGenSig};
pub use self::sty::{InferTy, ParamTy, ProjectionTy, ExistentialPredicate};
pub use self::sty::{ClosureSubsts, GeneratorSubsts, UpvarSubsts, TypeAndMut};
pub use self::sty::{TraitRef, TypeVariants, PolyTraitRef};
pub use self::sty::{ExistentialTraitRef, PolyExistentialTraitRef};
pub use self::sty::{ExistentialProjection, PolyExistentialProjection, Const};
pub use self::sty::{BoundRegion, EarlyBoundRegion, FreeRegion, Region};
pub use self::sty::RegionKind;
pub use self::sty::{TyVid, IntVid, FloatVid, RegionVid};
pub use self::sty::BoundRegion::*;
pub use self::sty::InferTy::*;
pub use self::sty::RegionKind::*;
pub use self::sty::TypeVariants::*;

pub use self::binding::BindingMode;
pub use self::binding::BindingMode::*;

pub use self::context::{TyCtxt, GlobalArenas, AllArenas, tls, keep_local};
pub use self::context::{Lift, TypeckTables};

pub use self::instance::{Instance, InstanceDef};

pub use self::trait_def::TraitDef;

pub use self::query::queries;

pub mod adjustment;
pub mod binding;
pub mod cast;
#[macro_use]
pub mod codec;
pub mod error;
mod erase_regions;
pub mod fast_reject;
pub mod fold;
pub mod inhabitedness;
pub mod item_path;
pub mod layout;
pub mod _match;
pub mod outlives;
pub mod query;
pub mod relate;
pub mod steal;
pub mod subst;
pub mod trait_def;
pub mod walk;
pub mod wf;
pub mod util;

mod context;
mod flags;
mod instance;
mod structural_impls;
mod sty;

// Data types

/// The complete set of all analyses described in this module. This is
/// produced by the driver and fed to codegen and later passes.
///
/// NB: These contents are being migrated into queries using the
/// *on-demand* infrastructure.
#[derive(Clone)]
pub struct CrateAnalysis {
    pub access_levels: Lrc<AccessLevels>,
    pub name: String,
    pub glob_map: Option<hir::GlobMap>,
}

#[derive(Clone)]
pub struct Resolutions {
    pub freevars: FreevarMap,
    pub trait_map: TraitMap,
    pub maybe_unused_trait_imports: NodeSet,
    pub maybe_unused_extern_crates: Vec<(NodeId, Span)>,
    pub export_map: ExportMap,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssociatedItemContainer {
    TraitContainer(DefId),
    ImplContainer(DefId),
}

impl AssociatedItemContainer {
    /// Asserts that this is the def-id of an associated item declared
    /// in a trait, and returns the trait def-id.
    pub fn assert_trait(&self) -> DefId {
        match *self {
            TraitContainer(id) => id,
            _ => bug!("associated item has wrong container type: {:?}", self)
        }
    }

    pub fn id(&self) -> DefId {
        match *self {
            TraitContainer(id) => id,
            ImplContainer(id) => id,
        }
    }
}

/// The "header" of an impl is everything outside the body: a Self type, a trait
/// ref (in the case of a trait impl), and a set of predicates (from the
/// bounds/where clauses).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ImplHeader<'tcx> {
    pub impl_def_id: DefId,
    pub self_ty: Ty<'tcx>,
    pub trait_ref: Option<TraitRef<'tcx>>,
    pub predicates: Vec<Predicate<'tcx>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AssociatedItem {
    pub def_id: DefId,
    pub name: Name,
    pub kind: AssociatedKind,
    pub vis: Visibility,
    pub defaultness: hir::Defaultness,
    pub container: AssociatedItemContainer,

    /// Whether this is a method with an explicit self
    /// as its first argument, allowing method calls.
    pub method_has_self_argument: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, RustcEncodable, RustcDecodable)]
pub enum AssociatedKind {
    Const,
    Method,
    Type
}

impl AssociatedItem {
    pub fn def(&self) -> Def {
        match self.kind {
            AssociatedKind::Const => Def::AssociatedConst(self.def_id),
            AssociatedKind::Method => Def::Method(self.def_id),
            AssociatedKind::Type => Def::AssociatedTy(self.def_id),
        }
    }

    /// Tests whether the associated item admits a non-trivial implementation
    /// for !
    pub fn relevant_for_never<'tcx>(&self) -> bool {
        match self.kind {
            AssociatedKind::Const => true,
            AssociatedKind::Type => true,
            // FIXME(canndrew): Be more thorough here, check if any argument is uninhabited.
            AssociatedKind::Method => !self.method_has_self_argument,
        }
    }

    pub fn signature<'a, 'tcx>(&self, tcx: &TyCtxt<'a, 'tcx, 'tcx>) -> String {
        match self.kind {
            ty::AssociatedKind::Method => {
                // We skip the binder here because the binder would deanonymize all
                // late-bound regions, and we don't want method signatures to show up
                // `as for<'r> fn(&'r MyType)`.  Pretty-printing handles late-bound
                // regions just fine, showing `fn(&MyType)`.
                format!("{}", tcx.fn_sig(self.def_id).skip_binder())
            }
            ty::AssociatedKind::Type => format!("type {};", self.name.to_string()),
            ty::AssociatedKind::Const => {
                format!("const {}: {:?};", self.name.to_string(), tcx.type_of(self.def_id))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, RustcEncodable, RustcDecodable)]
pub enum Visibility {
    /// Visible everywhere (including in other crates).
    Public,
    /// Visible only in the given crate-local module.
    Restricted(DefId),
    /// Not visible anywhere in the local crate. This is the visibility of private external items.
    Invisible,
}

pub trait DefIdTree: Copy {
    fn parent(self, id: DefId) -> Option<DefId>;

    fn is_descendant_of(self, mut descendant: DefId, ancestor: DefId) -> bool {
        if descendant.krate != ancestor.krate {
            return false;
        }

        while descendant != ancestor {
            match self.parent(descendant) {
                Some(parent) => descendant = parent,
                None => return false,
            }
        }
        true
    }
}

impl<'a, 'gcx, 'tcx> DefIdTree for TyCtxt<'a, 'gcx, 'tcx> {
    fn parent(self, id: DefId) -> Option<DefId> {
        self.def_key(id).parent.map(|index| DefId { index: index, ..id })
    }
}

impl Visibility {
    pub fn from_hir(visibility: &hir::Visibility, id: NodeId, tcx: TyCtxt) -> Self {
        match *visibility {
            hir::Public => Visibility::Public,
            hir::Visibility::Crate(_) => Visibility::Restricted(DefId::local(CRATE_DEF_INDEX)),
            hir::Visibility::Restricted { ref path, .. } => match path.def {
                // If there is no resolution, `resolve` will have already reported an error, so
                // assume that the visibility is public to avoid reporting more privacy errors.
                Def::Err => Visibility::Public,
                def => Visibility::Restricted(def.def_id()),
            },
            hir::Inherited => {
                Visibility::Restricted(tcx.hir.get_module_parent(id))
            }
        }
    }

    /// Returns true if an item with this visibility is accessible from the given block.
    pub fn is_accessible_from<T: DefIdTree>(self, module: DefId, tree: T) -> bool {
        let restriction = match self {
            // Public items are visible everywhere.
            Visibility::Public => return true,
            // Private items from other crates are visible nowhere.
            Visibility::Invisible => return false,
            // Restricted items are visible in an arbitrary local module.
            Visibility::Restricted(other) if other.krate != module.krate => return false,
            Visibility::Restricted(module) => module,
        };

        tree.is_descendant_of(module, restriction)
    }

    /// Returns true if this visibility is at least as accessible as the given visibility
    pub fn is_at_least<T: DefIdTree>(self, vis: Visibility, tree: T) -> bool {
        let vis_restriction = match vis {
            Visibility::Public => return self == Visibility::Public,
            Visibility::Invisible => return true,
            Visibility::Restricted(module) => module,
        };

        self.is_accessible_from(vis_restriction, tree)
    }

    // Returns true if this item is visible anywhere in the local crate.
    pub fn is_visible_locally(self) -> bool {
        match self {
            Visibility::Public => true,
            Visibility::Restricted(def_id) => def_id.is_local(),
            Visibility::Invisible => false,
        }
    }
}

#[derive(Clone, PartialEq, RustcDecodable, RustcEncodable, Copy)]
pub enum Variance {
    Covariant,      // T<A> <: T<B> iff A <: B -- e.g., function return type
    Invariant,      // T<A> <: T<B> iff B == A -- e.g., type of mutable cell
    Contravariant,  // T<A> <: T<B> iff B <: A -- e.g., function param type
    Bivariant,      // T<A> <: T<B>            -- e.g., unused type parameter
}

/// The crate variances map is computed during typeck and contains the
/// variance of every item in the local crate. You should not use it
/// directly, because to do so will make your pass dependent on the
/// HIR of every item in the local crate. Instead, use
/// `tcx.variances_of()` to get the variance for a *particular*
/// item.
pub struct CrateVariancesMap {
    /// For each item with generics, maps to a vector of the variance
    /// of its generics.  If an item has no generics, it will have no
    /// entry.
    pub variances: FxHashMap<DefId, Lrc<Vec<ty::Variance>>>,

    /// An empty vector, useful for cloning.
    pub empty_variance: Lrc<Vec<ty::Variance>>,
}

impl Variance {
    /// `a.xform(b)` combines the variance of a context with the
    /// variance of a type with the following meaning.  If we are in a
    /// context with variance `a`, and we encounter a type argument in
    /// a position with variance `b`, then `a.xform(b)` is the new
    /// variance with which the argument appears.
    ///
    /// Example 1:
    ///
    ///     *mut Vec<i32>
    ///
    /// Here, the "ambient" variance starts as covariant. `*mut T` is
    /// invariant with respect to `T`, so the variance in which the
    /// `Vec<i32>` appears is `Covariant.xform(Invariant)`, which
    /// yields `Invariant`. Now, the type `Vec<T>` is covariant with
    /// respect to its type argument `T`, and hence the variance of
    /// the `i32` here is `Invariant.xform(Covariant)`, which results
    /// (again) in `Invariant`.
    ///
    /// Example 2:
    ///
    ///     fn(*const Vec<i32>, *mut Vec<i32)
    ///
    /// The ambient variance is covariant. A `fn` type is
    /// contravariant with respect to its parameters, so the variance
    /// within which both pointer types appear is
    /// `Covariant.xform(Contravariant)`, or `Contravariant`.  `*const
    /// T` is covariant with respect to `T`, so the variance within
    /// which the first `Vec<i32>` appears is
    /// `Contravariant.xform(Covariant)` or `Contravariant`.  The same
    /// is true for its `i32` argument. In the `*mut T` case, the
    /// variance of `Vec<i32>` is `Contravariant.xform(Invariant)`,
    /// and hence the outermost type is `Invariant` with respect to
    /// `Vec<i32>` (and its `i32` argument).
    ///
    /// Source: Figure 1 of "Taming the Wildcards:
    /// Combining Definition- and Use-Site Variance" published in PLDI'11.
    pub fn xform(self, v: ty::Variance) -> ty::Variance {
        match (self, v) {
            // Figure 1, column 1.
            (ty::Covariant, ty::Covariant) => ty::Covariant,
            (ty::Covariant, ty::Contravariant) => ty::Contravariant,
            (ty::Covariant, ty::Invariant) => ty::Invariant,
            (ty::Covariant, ty::Bivariant) => ty::Bivariant,

            // Figure 1, column 2.
            (ty::Contravariant, ty::Covariant) => ty::Contravariant,
            (ty::Contravariant, ty::Contravariant) => ty::Covariant,
            (ty::Contravariant, ty::Invariant) => ty::Invariant,
            (ty::Contravariant, ty::Bivariant) => ty::Bivariant,

            // Figure 1, column 3.
            (ty::Invariant, _) => ty::Invariant,

            // Figure 1, column 4.
            (ty::Bivariant, _) => ty::Bivariant,
        }
    }
}

// Contains information needed to resolve types and (in the future) look up
// the types of AST nodes.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CReaderCacheKey {
    pub cnum: CrateNum,
    pub pos: usize,
}

// Flags that we track on types. These flags are propagated upwards
// through the type during type construction, so that we can quickly
// check whether the type has various kinds of types in it without
// recursing over the type itself.
bitflags! {
    pub struct TypeFlags: u32 {
        const HAS_PARAMS         = 1 << 0;
        const HAS_SELF           = 1 << 1;
        const HAS_TY_INFER       = 1 << 2;
        const HAS_RE_INFER       = 1 << 3;
        const HAS_RE_SKOL        = 1 << 4;

        /// Does this have any `ReEarlyBound` regions? Used to
        /// determine whether substitition is required, since those
        /// represent regions that are bound in a `ty::Generics` and
        /// hence may be substituted.
        const HAS_RE_EARLY_BOUND = 1 << 5;

        /// Does this have any region that "appears free" in the type?
        /// Basically anything but `ReLateBound` and `ReErased`.
        const HAS_FREE_REGIONS   = 1 << 6;

        /// Is an error type reachable?
        const HAS_TY_ERR         = 1 << 7;
        const HAS_PROJECTION     = 1 << 8;

        // FIXME: Rename this to the actual property since it's used for generators too
        const HAS_TY_CLOSURE     = 1 << 9;

        // true if there are "names" of types and regions and so forth
        // that are local to a particular fn
        const HAS_FREE_LOCAL_NAMES    = 1 << 10;

        // Present if the type belongs in a local type context.
        // Only set for TyInfer other than Fresh.
        const KEEP_IN_LOCAL_TCX  = 1 << 11;

        // Is there a projection that does not involve a bound region?
        // Currently we can't normalize projections w/ bound regions.
        const HAS_NORMALIZABLE_PROJECTION = 1 << 12;

        // Set if this includes a "canonical" type or region var --
        // ought to be true only for the results of canonicalization.
        const HAS_CANONICAL_VARS = 1 << 13;

        /// Does this have any `ReLateBound` regions? Used to check
        /// if a global bound is safe to evaluate.
        const HAS_RE_LATE_BOUND = 1 << 14;

        const NEEDS_SUBST        = TypeFlags::HAS_PARAMS.bits |
                                   TypeFlags::HAS_SELF.bits |
                                   TypeFlags::HAS_RE_EARLY_BOUND.bits;

        // Flags representing the nominal content of a type,
        // computed by FlagsComputation. If you add a new nominal
        // flag, it should be added here too.
        const NOMINAL_FLAGS     = TypeFlags::HAS_PARAMS.bits |
                                  TypeFlags::HAS_SELF.bits |
                                  TypeFlags::HAS_TY_INFER.bits |
                                  TypeFlags::HAS_RE_INFER.bits |
                                  TypeFlags::HAS_RE_SKOL.bits |
                                  TypeFlags::HAS_RE_EARLY_BOUND.bits |
                                  TypeFlags::HAS_FREE_REGIONS.bits |
                                  TypeFlags::HAS_TY_ERR.bits |
                                  TypeFlags::HAS_PROJECTION.bits |
                                  TypeFlags::HAS_TY_CLOSURE.bits |
                                  TypeFlags::HAS_FREE_LOCAL_NAMES.bits |
                                  TypeFlags::KEEP_IN_LOCAL_TCX.bits |
                                  TypeFlags::HAS_CANONICAL_VARS.bits |
                                  TypeFlags::HAS_RE_LATE_BOUND.bits;
    }
}

pub struct TyS<'tcx> {
    pub sty: TypeVariants<'tcx>,
    pub flags: TypeFlags,

    /// This is a kind of confusing thing: it stores the smallest
    /// binder such that
    ///
    /// (a) the binder itself captures nothing but
    /// (b) all the late-bound things within the type are captured
    ///     by some sub-binder.
    ///
    /// So, for a type without any late-bound things, like `u32`, this
    /// will be INNERMOST, because that is the innermost binder that
    /// captures nothing. But for a type `&'D u32`, where `'D` is a
    /// late-bound region with debruijn index D, this would be D+1 --
    /// the binder itself does not capture D, but D is captured by an
    /// inner binder.
    ///
    /// We call this concept an "exclusive" binder D (because all
    /// debruijn indices within the type are contained within `0..D`
    /// (exclusive)).
    outer_exclusive_binder: ty::DebruijnIndex,
}

impl<'tcx> Ord for TyS<'tcx> {
    fn cmp(&self, other: &TyS<'tcx>) -> Ordering {
        self.sty.cmp(&other.sty)
    }
}

impl<'tcx> PartialOrd for TyS<'tcx> {
    fn partial_cmp(&self, other: &TyS<'tcx>) -> Option<Ordering> {
        Some(self.sty.cmp(&other.sty))
    }
}

impl<'tcx> PartialEq for TyS<'tcx> {
    #[inline]
    fn eq(&self, other: &TyS<'tcx>) -> bool {
        // (self as *const _) == (other as *const _)
        (self as *const TyS<'tcx>) == (other as *const TyS<'tcx>)
    }
}
impl<'tcx> Eq for TyS<'tcx> {}

impl<'tcx> Hash for TyS<'tcx> {
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self as *const TyS).hash(s)
    }
}

impl<'tcx> TyS<'tcx> {
    pub fn is_primitive_ty(&self) -> bool {
        match self.sty {
            TypeVariants::TyBool |
                TypeVariants::TyChar |
                TypeVariants::TyInt(_) |
                TypeVariants::TyUint(_) |
                TypeVariants::TyFloat(_) |
                TypeVariants::TyInfer(InferTy::IntVar(_)) |
                TypeVariants::TyInfer(InferTy::FloatVar(_)) |
                TypeVariants::TyInfer(InferTy::FreshIntTy(_)) |
                TypeVariants::TyInfer(InferTy::FreshFloatTy(_)) => true,
            TypeVariants::TyRef(_, x, _) => x.is_primitive_ty(),
            _ => false,
        }
    }

    pub fn is_suggestable(&self) -> bool {
        match self.sty {
            TypeVariants::TyAnon(..) |
            TypeVariants::TyFnDef(..) |
            TypeVariants::TyFnPtr(..) |
            TypeVariants::TyDynamic(..) |
            TypeVariants::TyClosure(..) |
            TypeVariants::TyInfer(..) |
            TypeVariants::TyProjection(..) => false,
            _ => true,
        }
    }
}

impl<'a, 'gcx> HashStable<StableHashingContext<'a>> for ty::TyS<'gcx> {
    fn hash_stable<W: StableHasherResult>(&self,
                                          hcx: &mut StableHashingContext<'a>,
                                          hasher: &mut StableHasher<W>) {
        let ty::TyS {
            ref sty,

            // The other fields just provide fast access to information that is
            // also contained in `sty`, so no need to hash them.
            flags: _,

            outer_exclusive_binder: _,
        } = *self;

        sty.hash_stable(hcx, hasher);
    }
}

pub type Ty<'tcx> = &'tcx TyS<'tcx>;

impl<'tcx> serialize::UseSpecializedEncodable for Ty<'tcx> {}
impl<'tcx> serialize::UseSpecializedDecodable for Ty<'tcx> {}

pub type CanonicalTy<'gcx> = Canonical<'gcx, Ty<'gcx>>;

impl <'gcx: 'tcx, 'tcx> Canonicalize<'gcx, 'tcx> for Ty<'tcx> {
    type Canonicalized = CanonicalTy<'gcx>;

    fn intern(_gcx: TyCtxt<'_, 'gcx, 'gcx>,
              value: Canonical<'gcx, Self::Lifted>) -> Self::Canonicalized {
        value
    }
}

extern {
    /// A dummy type used to force Slice to by unsized without requiring fat pointers
    type OpaqueSliceContents;
}

/// A wrapper for slices with the additional invariant
/// that the slice is interned and no other slice with
/// the same contents can exist in the same context.
/// This means we can use pointer for both
/// equality comparisons and hashing.
#[repr(C)]
pub struct Slice<T> {
    len: usize,
    data: [T; 0],
    opaque: OpaqueSliceContents,
}

unsafe impl<T: Sync> Sync for Slice<T> {}

impl<T: Copy> Slice<T> {
    #[inline]
    fn from_arena<'tcx>(arena: &'tcx SyncDroplessArena, slice: &[T]) -> &'tcx Slice<T> {
        assert!(!mem::needs_drop::<T>());
        assert!(mem::size_of::<T>() != 0);
        assert!(slice.len() != 0);

        // Align up the size of the len (usize) field
        let align = mem::align_of::<T>();
        let align_mask = align - 1;
        let offset = mem::size_of::<usize>();
        let offset = (offset + align_mask) & !align_mask;

        let size = offset + slice.len() * mem::size_of::<T>();

        let mem = arena.alloc_raw(
            size,
            cmp::max(mem::align_of::<T>(), mem::align_of::<usize>()));
        unsafe {
            let result = &mut *(mem.as_mut_ptr() as *mut Slice<T>);
            // Write the length
            result.len = slice.len();

            // Write the elements
            let arena_slice = slice::from_raw_parts_mut(result.data.as_mut_ptr(), result.len);
            arena_slice.copy_from_slice(slice);

            result
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Slice<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: Encodable> Encodable for Slice<T> {
    #[inline]
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        (**self).encode(s)
    }
}

impl<T> Ord for Slice<T> where T: Ord {
    fn cmp(&self, other: &Slice<T>) -> Ordering {
        if self == other { Ordering::Equal } else {
            <[T] as Ord>::cmp(&**self, &**other)
        }
    }
}

impl<T> PartialOrd for Slice<T> where T: PartialOrd {
    fn partial_cmp(&self, other: &Slice<T>) -> Option<Ordering> {
        if self == other { Some(Ordering::Equal) } else {
            <[T] as PartialOrd>::partial_cmp(&**self, &**other)
        }
    }
}

impl<T: PartialEq> PartialEq for Slice<T> {
    #[inline]
    fn eq(&self, other: &Slice<T>) -> bool {
        (self as *const _) == (other as *const _)
    }
}
impl<T: Eq> Eq for Slice<T> {}

impl<T> Hash for Slice<T> {
    #[inline]
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self as *const Slice<T>).hash(s)
    }
}

impl<T> Deref for Slice<T> {
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.data.as_ptr(), self.len)
        }
    }
}

impl<'a, T> IntoIterator for &'a Slice<T> {
    type Item = &'a T;
    type IntoIter = <&'a [T] as IntoIterator>::IntoIter;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self[..].iter()
    }
}

impl<'tcx> serialize::UseSpecializedDecodable for &'tcx Slice<Ty<'tcx>> {}

impl<T> Slice<T> {
    #[inline(always)]
    pub fn empty<'a>() -> &'a Slice<T> {
        #[repr(align(64), C)]
        struct EmptySlice([u8; 64]);
        static EMPTY_SLICE: EmptySlice = EmptySlice([0; 64]);
        assert!(mem::align_of::<T>() <= 64);
        unsafe {
            &*(&EMPTY_SLICE as *const _ as *const Slice<T>)
        }
    }
}

/// Upvars do not get their own node-id. Instead, we use the pair of
/// the original var id (that is, the root variable that is referenced
/// by the upvar) and the id of the closure expression.
#[derive(Clone, Copy, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct UpvarId {
    pub var_id: hir::HirId,
    pub closure_expr_id: LocalDefId,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable, Copy)]
pub enum BorrowKind {
    /// Data must be immutable and is aliasable.
    ImmBorrow,

    /// Data must be immutable but not aliasable.  This kind of borrow
    /// cannot currently be expressed by the user and is used only in
    /// implicit closure bindings. It is needed when the closure
    /// is borrowing or mutating a mutable referent, e.g.:
    ///
    ///    let x: &mut isize = ...;
    ///    let y = || *x += 5;
    ///
    /// If we were to try to translate this closure into a more explicit
    /// form, we'd encounter an error with the code as written:
    ///
    ///    struct Env { x: & &mut isize }
    ///    let x: &mut isize = ...;
    ///    let y = (&mut Env { &x }, fn_ptr);  // Closure is pair of env and fn
    ///    fn fn_ptr(env: &mut Env) { **env.x += 5; }
    ///
    /// This is then illegal because you cannot mutate a `&mut` found
    /// in an aliasable location. To solve, you'd have to translate with
    /// an `&mut` borrow:
    ///
    ///    struct Env { x: & &mut isize }
    ///    let x: &mut isize = ...;
    ///    let y = (&mut Env { &mut x }, fn_ptr); // changed from &x to &mut x
    ///    fn fn_ptr(env: &mut Env) { **env.x += 5; }
    ///
    /// Now the assignment to `**env.x` is legal, but creating a
    /// mutable pointer to `x` is not because `x` is not mutable. We
    /// could fix this by declaring `x` as `let mut x`. This is ok in
    /// user code, if awkward, but extra weird for closures, since the
    /// borrow is hidden.
    ///
    /// So we introduce a "unique imm" borrow -- the referent is
    /// immutable, but not aliasable. This solves the problem. For
    /// simplicity, we don't give users the way to express this
    /// borrow, it's just used when translating closures.
    UniqueImmBorrow,

    /// Data is mutable and not aliasable.
    MutBorrow
}

/// Information describing the capture of an upvar. This is computed
/// during `typeck`, specifically by `regionck`.
#[derive(PartialEq, Clone, Debug, Copy, RustcEncodable, RustcDecodable)]
pub enum UpvarCapture<'tcx> {
    /// Upvar is captured by value. This is always true when the
    /// closure is labeled `move`, but can also be true in other cases
    /// depending on inference.
    ByValue,

    /// Upvar is captured by reference.
    ByRef(UpvarBorrow<'tcx>),
}

#[derive(PartialEq, Clone, Copy, RustcEncodable, RustcDecodable)]
pub struct UpvarBorrow<'tcx> {
    /// The kind of borrow: by-ref upvars have access to shared
    /// immutable borrows, which are not part of the normal language
    /// syntax.
    pub kind: BorrowKind,

    /// Region of the resulting reference.
    pub region: ty::Region<'tcx>,
}

pub type UpvarCaptureMap<'tcx> = FxHashMap<UpvarId, UpvarCapture<'tcx>>;

#[derive(Copy, Clone)]
pub struct ClosureUpvar<'tcx> {
    pub def: Def,
    pub span: Span,
    pub ty: Ty<'tcx>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IntVarValue {
    IntType(ast::IntTy),
    UintType(ast::UintTy),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FloatVarValue(pub ast::FloatTy);

impl ty::EarlyBoundRegion {
    pub fn to_bound_region(&self) -> ty::BoundRegion {
        ty::BoundRegion::BrNamed(self.def_id, self.name)
    }
}

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub enum GenericParamDefKind {
    Lifetime,
    Type {
        has_default: bool,
        object_lifetime_default: ObjectLifetimeDefault,
        synthetic: Option<hir::SyntheticTyParamKind>,
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct GenericParamDef {
    pub name: InternedString,
    pub def_id: DefId,
    pub index: u32,

    /// `pure_wrt_drop`, set by the (unsafe) `#[may_dangle]` attribute
    /// on generic parameter `'a`/`T`, asserts data behind the parameter
    /// `'a`/`T` won't be accessed during the parent type's `Drop` impl.
    pub pure_wrt_drop: bool,

    pub kind: GenericParamDefKind,
}

impl GenericParamDef {
    pub fn to_early_bound_region_data(&self) -> ty::EarlyBoundRegion {
        match self.kind {
            GenericParamDefKind::Lifetime => {
                ty::EarlyBoundRegion {
                    def_id: self.def_id,
                    index: self.index,
                    name: self.name,
                }
            }
            _ => bug!("cannot convert a non-lifetime parameter def to an early bound region")
        }
    }

    pub fn to_bound_region(&self) -> ty::BoundRegion {
        match self.kind {
            GenericParamDefKind::Lifetime => {
                self.to_early_bound_region_data().to_bound_region()
            }
            _ => bug!("cannot convert a non-lifetime parameter def to an early bound region")
        }
    }
}

pub struct GenericParamCount {
    pub lifetimes: usize,
    pub types: usize,
}

/// Information about the formal type/lifetime parameters associated
/// with an item or method. Analogous to hir::Generics.
///
/// The ordering of parameters is the same as in Subst (excluding child generics):
/// Self (optionally), Lifetime params..., Type params...
#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct Generics {
    pub parent: Option<DefId>,
    pub parent_count: usize,
    pub params: Vec<GenericParamDef>,

    /// Reverse map to the `index` field of each `GenericParamDef`
    pub param_def_id_to_index: FxHashMap<DefId, u32>,

    pub has_self: bool,
    pub has_late_bound_regions: Option<Span>,
}

impl<'a, 'gcx, 'tcx> Generics {
    pub fn count(&self) -> usize {
        self.parent_count + self.params.len()
    }

    pub fn own_counts(&self) -> GenericParamCount {
        // We could cache this as a property of `GenericParamCount`, but
        // the aim is to refactor this away entirely eventually and the
        // presence of this method will be a constant reminder.
        let mut own_counts = GenericParamCount {
            lifetimes: 0,
            types: 0,
        };

        for param in &self.params {
            match param.kind {
                GenericParamDefKind::Lifetime => own_counts.lifetimes += 1,
                GenericParamDefKind::Type {..} => own_counts.types += 1,
            };
        }

        own_counts
    }

    pub fn requires_monomorphization(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> bool {
        for param in &self.params {
            match param.kind {
                GenericParamDefKind::Type {..} => return true,
                GenericParamDefKind::Lifetime => {}
            }
        }
        if let Some(parent_def_id) = self.parent {
            let parent = tcx.generics_of(parent_def_id);
            parent.requires_monomorphization(tcx)
        } else {
            false
        }
    }

    pub fn region_param(&'tcx self,
                        param: &EarlyBoundRegion,
                        tcx: TyCtxt<'a, 'gcx, 'tcx>)
                        -> &'tcx GenericParamDef
    {
        if let Some(index) = param.index.checked_sub(self.parent_count as u32) {
            let param = &self.params[index as usize];
            match param.kind {
                ty::GenericParamDefKind::Lifetime => param,
                _ => bug!("expected lifetime parameter, but found another generic parameter")
            }
        } else {
            tcx.generics_of(self.parent.expect("parent_count>0 but no parent?"))
                .region_param(param, tcx)
        }
    }

    /// Returns the `GenericParamDef` associated with this `ParamTy`.
    pub fn type_param(&'tcx self,
                      param: &ParamTy,
                      tcx: TyCtxt<'a, 'gcx, 'tcx>)
                      -> &'tcx GenericParamDef {
        if let Some(index) = param.idx.checked_sub(self.parent_count as u32) {
            let param = &self.params[index as usize];
            match param.kind {
                ty::GenericParamDefKind::Type {..} => param,
                _ => bug!("expected type parameter, but found another generic parameter")
            }
        } else {
            tcx.generics_of(self.parent.expect("parent_count>0 but no parent?"))
                .type_param(param, tcx)
        }
    }
}

/// Bounds on generics.
#[derive(Clone, Default)]
pub struct GenericPredicates<'tcx> {
    pub parent: Option<DefId>,
    pub predicates: Vec<Predicate<'tcx>>,
}

impl<'tcx> serialize::UseSpecializedEncodable for GenericPredicates<'tcx> {}
impl<'tcx> serialize::UseSpecializedDecodable for GenericPredicates<'tcx> {}

impl<'a, 'gcx, 'tcx> GenericPredicates<'tcx> {
    pub fn instantiate(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, substs: &Substs<'tcx>)
                       -> InstantiatedPredicates<'tcx> {
        let mut instantiated = InstantiatedPredicates::empty();
        self.instantiate_into(tcx, &mut instantiated, substs);
        instantiated
    }
    pub fn instantiate_own(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, substs: &Substs<'tcx>)
                           -> InstantiatedPredicates<'tcx> {
        InstantiatedPredicates {
            predicates: self.predicates.subst(tcx, substs)
        }
    }

    fn instantiate_into(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                        instantiated: &mut InstantiatedPredicates<'tcx>,
                        substs: &Substs<'tcx>) {
        if let Some(def_id) = self.parent {
            tcx.predicates_of(def_id).instantiate_into(tcx, instantiated, substs);
        }
        instantiated.predicates.extend(self.predicates.iter().map(|p| p.subst(tcx, substs)))
    }

    pub fn instantiate_identity(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>)
                                -> InstantiatedPredicates<'tcx> {
        let mut instantiated = InstantiatedPredicates::empty();
        self.instantiate_identity_into(tcx, &mut instantiated);
        instantiated
    }

    fn instantiate_identity_into(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                 instantiated: &mut InstantiatedPredicates<'tcx>) {
        if let Some(def_id) = self.parent {
            tcx.predicates_of(def_id).instantiate_identity_into(tcx, instantiated);
        }
        instantiated.predicates.extend(&self.predicates)
    }

    pub fn instantiate_supertrait(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                  poly_trait_ref: &ty::PolyTraitRef<'tcx>)
                                  -> InstantiatedPredicates<'tcx>
    {
        assert_eq!(self.parent, None);
        InstantiatedPredicates {
            predicates: self.predicates.iter().map(|pred| {
                pred.subst_supertrait(tcx, poly_trait_ref)
            }).collect()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub enum Predicate<'tcx> {
    /// Corresponds to `where Foo : Bar<A,B,C>`. `Foo` here would be
    /// the `Self` type of the trait reference and `A`, `B`, and `C`
    /// would be the type parameters.
    Trait(PolyTraitPredicate<'tcx>),

    /// where 'a : 'b
    RegionOutlives(PolyRegionOutlivesPredicate<'tcx>),

    /// where T : 'a
    TypeOutlives(PolyTypeOutlivesPredicate<'tcx>),

    /// where <T as TraitRef>::Name == X, approximately.
    /// See `ProjectionPredicate` struct for details.
    Projection(PolyProjectionPredicate<'tcx>),

    /// no syntax: T WF
    WellFormed(Ty<'tcx>),

    /// trait must be object-safe
    ObjectSafe(DefId),

    /// No direct syntax. May be thought of as `where T : FnFoo<...>`
    /// for some substitutions `...` and T being a closure type.
    /// Satisfied (or refuted) once we know the closure's kind.
    ClosureKind(DefId, ClosureSubsts<'tcx>, ClosureKind),

    /// `T1 <: T2`
    Subtype(PolySubtypePredicate<'tcx>),

    /// Constant initializer must evaluate successfully.
    ConstEvaluatable(DefId, &'tcx Substs<'tcx>),
}

/// The crate outlives map is computed during typeck and contains the
/// outlives of every item in the local crate. You should not use it
/// directly, because to do so will make your pass dependent on the
/// HIR of every item in the local crate. Instead, use
/// `tcx.inferred_outlives_of()` to get the outlives for a *particular*
/// item.
pub struct CratePredicatesMap<'tcx> {
    /// For each struct with outlive bounds, maps to a vector of the
    /// predicate of its outlive bounds. If an item has no outlives
    /// bounds, it will have no entry.
    pub predicates: FxHashMap<DefId, Lrc<Vec<ty::Predicate<'tcx>>>>,

    /// An empty vector, useful for cloning.
    pub empty_predicate: Lrc<Vec<ty::Predicate<'tcx>>>,
}

impl<'tcx> AsRef<Predicate<'tcx>> for Predicate<'tcx> {
    fn as_ref(&self) -> &Predicate<'tcx> {
        self
    }
}

impl<'a, 'gcx, 'tcx> Predicate<'tcx> {
    /// Performs a substitution suitable for going from a
    /// poly-trait-ref to supertraits that must hold if that
    /// poly-trait-ref holds. This is slightly different from a normal
    /// substitution in terms of what happens with bound regions.  See
    /// lengthy comment below for details.
    pub fn subst_supertrait(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                            trait_ref: &ty::PolyTraitRef<'tcx>)
                            -> ty::Predicate<'tcx>
    {
        // The interaction between HRTB and supertraits is not entirely
        // obvious. Let me walk you (and myself) through an example.
        //
        // Let's start with an easy case. Consider two traits:
        //
        //     trait Foo<'a> : Bar<'a,'a> { }
        //     trait Bar<'b,'c> { }
        //
        // Now, if we have a trait reference `for<'x> T : Foo<'x>`, then
        // we can deduce that `for<'x> T : Bar<'x,'x>`. Basically, if we
        // knew that `Foo<'x>` (for any 'x) then we also know that
        // `Bar<'x,'x>` (for any 'x). This more-or-less falls out from
        // normal substitution.
        //
        // In terms of why this is sound, the idea is that whenever there
        // is an impl of `T:Foo<'a>`, it must show that `T:Bar<'a,'a>`
        // holds.  So if there is an impl of `T:Foo<'a>` that applies to
        // all `'a`, then we must know that `T:Bar<'a,'a>` holds for all
        // `'a`.
        //
        // Another example to be careful of is this:
        //
        //     trait Foo1<'a> : for<'b> Bar1<'a,'b> { }
        //     trait Bar1<'b,'c> { }
        //
        // Here, if we have `for<'x> T : Foo1<'x>`, then what do we know?
        // The answer is that we know `for<'x,'b> T : Bar1<'x,'b>`. The
        // reason is similar to the previous example: any impl of
        // `T:Foo1<'x>` must show that `for<'b> T : Bar1<'x, 'b>`.  So
        // basically we would want to collapse the bound lifetimes from
        // the input (`trait_ref`) and the supertraits.
        //
        // To achieve this in practice is fairly straightforward. Let's
        // consider the more complicated scenario:
        //
        // - We start out with `for<'x> T : Foo1<'x>`. In this case, `'x`
        //   has a De Bruijn index of 1. We want to produce `for<'x,'b> T : Bar1<'x,'b>`,
        //   where both `'x` and `'b` would have a DB index of 1.
        //   The substitution from the input trait-ref is therefore going to be
        //   `'a => 'x` (where `'x` has a DB index of 1).
        // - The super-trait-ref is `for<'b> Bar1<'a,'b>`, where `'a` is an
        //   early-bound parameter and `'b' is a late-bound parameter with a
        //   DB index of 1.
        // - If we replace `'a` with `'x` from the input, it too will have
        //   a DB index of 1, and thus we'll have `for<'x,'b> Bar1<'x,'b>`
        //   just as we wanted.
        //
        // There is only one catch. If we just apply the substitution `'a
        // => 'x` to `for<'b> Bar1<'a,'b>`, the substitution code will
        // adjust the DB index because we substituting into a binder (it
        // tries to be so smart...) resulting in `for<'x> for<'b>
        // Bar1<'x,'b>` (we have no syntax for this, so use your
        // imagination). Basically the 'x will have DB index of 2 and 'b
        // will have DB index of 1. Not quite what we want. So we apply
        // the substitution to the *contents* of the trait reference,
        // rather than the trait reference itself (put another way, the
        // substitution code expects equal binding levels in the values
        // from the substitution and the value being substituted into, and
        // this trick achieves that).

        let substs = &trait_ref.skip_binder().substs;
        match *self {
            Predicate::Trait(ref binder) =>
                Predicate::Trait(binder.map_bound(|data| data.subst(tcx, substs))),
            Predicate::Subtype(ref binder) =>
                Predicate::Subtype(binder.map_bound(|data| data.subst(tcx, substs))),
            Predicate::RegionOutlives(ref binder) =>
                Predicate::RegionOutlives(binder.map_bound(|data| data.subst(tcx, substs))),
            Predicate::TypeOutlives(ref binder) =>
                Predicate::TypeOutlives(binder.map_bound(|data| data.subst(tcx, substs))),
            Predicate::Projection(ref binder) =>
                Predicate::Projection(binder.map_bound(|data| data.subst(tcx, substs))),
            Predicate::WellFormed(data) =>
                Predicate::WellFormed(data.subst(tcx, substs)),
            Predicate::ObjectSafe(trait_def_id) =>
                Predicate::ObjectSafe(trait_def_id),
            Predicate::ClosureKind(closure_def_id, closure_substs, kind) =>
                Predicate::ClosureKind(closure_def_id, closure_substs.subst(tcx, substs), kind),
            Predicate::ConstEvaluatable(def_id, const_substs) =>
                Predicate::ConstEvaluatable(def_id, const_substs.subst(tcx, substs)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct TraitPredicate<'tcx> {
    pub trait_ref: TraitRef<'tcx>
}
pub type PolyTraitPredicate<'tcx> = ty::Binder<TraitPredicate<'tcx>>;

impl<'tcx> TraitPredicate<'tcx> {
    pub fn def_id(&self) -> DefId {
        self.trait_ref.def_id
    }

    pub fn input_types<'a>(&'a self) -> impl DoubleEndedIterator<Item=Ty<'tcx>> + 'a {
        self.trait_ref.input_types()
    }

    pub fn self_ty(&self) -> Ty<'tcx> {
        self.trait_ref.self_ty()
    }
}

impl<'tcx> PolyTraitPredicate<'tcx> {
    pub fn def_id(&self) -> DefId {
        // ok to skip binder since trait def-id does not care about regions
        self.skip_binder().def_id()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct OutlivesPredicate<A,B>(pub A, pub B); // `A : B`
pub type PolyOutlivesPredicate<A,B> = ty::Binder<OutlivesPredicate<A,B>>;
pub type RegionOutlivesPredicate<'tcx> = OutlivesPredicate<ty::Region<'tcx>,
                                                           ty::Region<'tcx>>;
pub type TypeOutlivesPredicate<'tcx> = OutlivesPredicate<Ty<'tcx>,
                                                         ty::Region<'tcx>>;
pub type PolyRegionOutlivesPredicate<'tcx> = ty::Binder<RegionOutlivesPredicate<'tcx>>;
pub type PolyTypeOutlivesPredicate<'tcx> = ty::Binder<TypeOutlivesPredicate<'tcx>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct SubtypePredicate<'tcx> {
    pub a_is_expected: bool,
    pub a: Ty<'tcx>,
    pub b: Ty<'tcx>
}
pub type PolySubtypePredicate<'tcx> = ty::Binder<SubtypePredicate<'tcx>>;

/// This kind of predicate has no *direct* correspondent in the
/// syntax, but it roughly corresponds to the syntactic forms:
///
/// 1. `T : TraitRef<..., Item=Type>`
/// 2. `<T as TraitRef<...>>::Item == Type` (NYI)
///
/// In particular, form #1 is "desugared" to the combination of a
/// normal trait predicate (`T : TraitRef<...>`) and one of these
/// predicates. Form #2 is a broader form in that it also permits
/// equality between arbitrary types. Processing an instance of
/// Form #2 eventually yields one of these `ProjectionPredicate`
/// instances to normalize the LHS.
#[derive(Copy, Clone, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct ProjectionPredicate<'tcx> {
    pub projection_ty: ProjectionTy<'tcx>,
    pub ty: Ty<'tcx>,
}

pub type PolyProjectionPredicate<'tcx> = Binder<ProjectionPredicate<'tcx>>;

impl<'tcx> PolyProjectionPredicate<'tcx> {
    /// Returns the def-id of the associated item being projected.
    pub fn item_def_id(&self) -> DefId {
        self.skip_binder().projection_ty.item_def_id
    }

    pub fn to_poly_trait_ref(&self, tcx: TyCtxt) -> PolyTraitRef<'tcx> {
        // Note: unlike with TraitRef::to_poly_trait_ref(),
        // self.0.trait_ref is permitted to have escaping regions.
        // This is because here `self` has a `Binder` and so does our
        // return value, so we are preserving the number of binding
        // levels.
        self.map_bound(|predicate| predicate.projection_ty.trait_ref(tcx))
    }

    pub fn ty(&self) -> Binder<Ty<'tcx>> {
        self.map_bound(|predicate| predicate.ty)
    }

    /// The DefId of the TraitItem for the associated type.
    ///
    /// Note that this is not the DefId of the TraitRef containing this
    /// associated type, which is in tcx.associated_item(projection_def_id()).container.
    pub fn projection_def_id(&self) -> DefId {
        // ok to skip binder since trait def-id does not care about regions
        self.skip_binder().projection_ty.item_def_id
    }
}

pub trait ToPolyTraitRef<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx>;
}

impl<'tcx> ToPolyTraitRef<'tcx> for TraitRef<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx> {
        ty::Binder::dummy(self.clone())
    }
}

impl<'tcx> ToPolyTraitRef<'tcx> for PolyTraitPredicate<'tcx> {
    fn to_poly_trait_ref(&self) -> PolyTraitRef<'tcx> {
        self.map_bound_ref(|trait_pred| trait_pred.trait_ref)
    }
}

pub trait ToPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx>;
}

impl<'tcx> ToPredicate<'tcx> for TraitRef<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        ty::Predicate::Trait(ty::Binder::dummy(ty::TraitPredicate {
            trait_ref: self.clone()
        }))
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyTraitRef<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        ty::Predicate::Trait(self.to_poly_trait_predicate())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyRegionOutlivesPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::RegionOutlives(self.clone())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyTypeOutlivesPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::TypeOutlives(self.clone())
    }
}

impl<'tcx> ToPredicate<'tcx> for PolyProjectionPredicate<'tcx> {
    fn to_predicate(&self) -> Predicate<'tcx> {
        Predicate::Projection(self.clone())
    }
}

impl<'tcx> Predicate<'tcx> {
    /// Iterates over the types in this predicate. Note that in all
    /// cases this is skipping over a binder, so late-bound regions
    /// with depth 0 are bound by the predicate.
    pub fn walk_tys(&self) -> IntoIter<Ty<'tcx>> {
        let vec: Vec<_> = match *self {
            ty::Predicate::Trait(ref data) => {
                data.skip_binder().input_types().collect()
            }
            ty::Predicate::Subtype(binder) => {
                let SubtypePredicate { a, b, a_is_expected: _ } = binder.skip_binder();
                vec![a, b]
            }
            ty::Predicate::TypeOutlives(binder) => {
                vec![binder.skip_binder().0]
            }
            ty::Predicate::RegionOutlives(..) => {
                vec![]
            }
            ty::Predicate::Projection(ref data) => {
                let inner = data.skip_binder();
                inner.projection_ty.substs.types().chain(Some(inner.ty)).collect()
            }
            ty::Predicate::WellFormed(data) => {
                vec![data]
            }
            ty::Predicate::ObjectSafe(_trait_def_id) => {
                vec![]
            }
            ty::Predicate::ClosureKind(_closure_def_id, closure_substs, _kind) => {
                closure_substs.substs.types().collect()
            }
            ty::Predicate::ConstEvaluatable(_, substs) => {
                substs.types().collect()
            }
        };

        // The only reason to collect into a vector here is that I was
        // too lazy to make the full (somewhat complicated) iterator
        // type that would be needed here. But I wanted this fn to
        // return an iterator conceptually, rather than a `Vec`, so as
        // to be closer to `Ty::walk`.
        vec.into_iter()
    }

    pub fn to_opt_poly_trait_ref(&self) -> Option<PolyTraitRef<'tcx>> {
        match *self {
            Predicate::Trait(ref t) => {
                Some(t.to_poly_trait_ref())
            }
            Predicate::Projection(..) |
            Predicate::Subtype(..) |
            Predicate::RegionOutlives(..) |
            Predicate::WellFormed(..) |
            Predicate::ObjectSafe(..) |
            Predicate::ClosureKind(..) |
            Predicate::TypeOutlives(..) |
            Predicate::ConstEvaluatable(..) => {
                None
            }
        }
    }

    pub fn to_opt_type_outlives(&self) -> Option<PolyTypeOutlivesPredicate<'tcx>> {
        match *self {
            Predicate::TypeOutlives(data) => {
                Some(data)
            }
            Predicate::Trait(..) |
            Predicate::Projection(..) |
            Predicate::Subtype(..) |
            Predicate::RegionOutlives(..) |
            Predicate::WellFormed(..) |
            Predicate::ObjectSafe(..) |
            Predicate::ClosureKind(..) |
            Predicate::ConstEvaluatable(..) => {
                None
            }
        }
    }
}

/// Represents the bounds declared on a particular set of type
/// parameters.  Should eventually be generalized into a flag list of
/// where clauses.  You can obtain a `InstantiatedPredicates` list from a
/// `GenericPredicates` by using the `instantiate` method. Note that this method
/// reflects an important semantic invariant of `InstantiatedPredicates`: while
/// the `GenericPredicates` are expressed in terms of the bound type
/// parameters of the impl/trait/whatever, an `InstantiatedPredicates` instance
/// represented a set of bounds for some particular instantiation,
/// meaning that the generic parameters have been substituted with
/// their values.
///
/// Example:
///
///     struct Foo<T,U:Bar<T>> { ... }
///
/// Here, the `GenericPredicates` for `Foo` would contain a list of bounds like
/// `[[], [U:Bar<T>]]`.  Now if there were some particular reference
/// like `Foo<isize,usize>`, then the `InstantiatedPredicates` would be `[[],
/// [usize:Bar<isize>]]`.
#[derive(Clone)]
pub struct InstantiatedPredicates<'tcx> {
    pub predicates: Vec<Predicate<'tcx>>,
}

impl<'tcx> InstantiatedPredicates<'tcx> {
    pub fn empty() -> InstantiatedPredicates<'tcx> {
        InstantiatedPredicates { predicates: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.predicates.is_empty()
    }
}

/// "Universes" are used during type- and trait-checking in the
/// presence of `for<..>` binders to control what sets of names are
/// visible. Universes are arranged into a tree: the root universe
/// contains names that are always visible. But when you enter into
/// some subuniverse, then it may add names that are only visible
/// within that subtree (but it can still name the names of its
/// ancestor universes).
///
/// To make this more concrete, consider this program:
///
/// ```
/// struct Foo { }
/// fn bar<T>(x: T) {
///   let y: for<'a> fn(&'a u8, Foo) = ...;
/// }
/// ```
///
/// The struct name `Foo` is in the root universe U0. But the type
/// parameter `T`, introduced on `bar`, is in a subuniverse U1 --
/// i.e., within `bar`, we can name both `T` and `Foo`, but outside of
/// `bar`, we cannot name `T`. Then, within the type of `y`, the
/// region `'a` is in a subuniverse U2 of U1, because we can name it
/// inside the fn type but not outside.
///
/// Universes are related to **skolemization** -- which is a way of
/// doing type- and trait-checking around these "forall" binders (also
/// called **universal quantification**). The idea is that when, in
/// the body of `bar`, we refer to `T` as a type, we aren't referring
/// to any type in particular, but rather a kind of "fresh" type that
/// is distinct from all other types we have actually declared. This
/// is called a **skolemized** type, and we use universes to talk
/// about this. In other words, a type name in universe 0 always
/// corresponds to some "ground" type that the user declared, but a
/// type name in a non-zero universe is a skolemized type -- an
/// idealized representative of "types in general" that we use for
/// checking generic functions.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct UniverseIndex(u32);

impl UniverseIndex {
    /// The root universe, where things that the user defined are
    /// visible.
    pub const ROOT: Self = UniverseIndex(0);

    /// A "subuniverse" corresponds to being inside a `forall` quantifier.
    /// So, for example, suppose we have this type in universe `U`:
    ///
    /// ```
    /// for<'a> fn(&'a u32)
    /// ```
    ///
    /// Once we "enter" into this `for<'a>` quantifier, we are in a
    /// subuniverse of `U` -- in this new universe, we can name the
    /// region `'a`, but that region was not nameable from `U` because
    /// it was not in scope there.
    pub fn subuniverse(self) -> UniverseIndex {
        UniverseIndex(self.0.checked_add(1).unwrap())
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for UniverseIndex {
    fn from(index: u32) -> Self {
        UniverseIndex(index)
    }
}

/// When type checking, we use the `ParamEnv` to track
/// details about the set of where-clauses that are in scope at this
/// particular point.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamEnv<'tcx> {
    /// Obligations that the caller must satisfy. This is basically
    /// the set of bounds on the in-scope type parameters, translated
    /// into Obligations, and elaborated and normalized.
    pub caller_bounds: &'tcx Slice<ty::Predicate<'tcx>>,

    /// Typically, this is `Reveal::UserFacing`, but during codegen we
    /// want `Reveal::All` -- note that this is always paired with an
    /// empty environment. To get that, use `ParamEnv::reveal()`.
    pub reveal: traits::Reveal,
}

impl<'tcx> ParamEnv<'tcx> {
    /// Construct a trait environment suitable for contexts where
    /// there are no where clauses in scope. Hidden types (like `impl
    /// Trait`) are left hidden, so this is suitable for ordinary
    /// type-checking.
    pub fn empty() -> Self {
        Self::new(ty::Slice::empty(), Reveal::UserFacing)
    }

    /// Construct a trait environment with no where clauses in scope
    /// where the values of all `impl Trait` and other hidden types
    /// are revealed. This is suitable for monomorphized, post-typeck
    /// environments like codegen or doing optimizations.
    ///
    /// NB. If you want to have predicates in scope, use `ParamEnv::new`,
    /// or invoke `param_env.with_reveal_all()`.
    pub fn reveal_all() -> Self {
        Self::new(ty::Slice::empty(), Reveal::All)
    }

    /// Construct a trait environment with the given set of predicates.
    pub fn new(caller_bounds: &'tcx ty::Slice<ty::Predicate<'tcx>>,
               reveal: Reveal)
               -> Self {
        ty::ParamEnv { caller_bounds, reveal }
    }

    /// Returns a new parameter environment with the same clauses, but
    /// which "reveals" the true results of projections in all cases
    /// (even for associated types that are specializable).  This is
    /// the desired behavior during codegen and certain other special
    /// contexts; normally though we want to use `Reveal::UserFacing`,
    /// which is the default.
    pub fn with_reveal_all(self) -> Self {
        ty::ParamEnv { reveal: Reveal::All, ..self }
    }

    /// Returns this same environment but with no caller bounds.
    pub fn without_caller_bounds(self) -> Self {
        ty::ParamEnv { caller_bounds: ty::Slice::empty(), ..self }
    }

    /// Creates a suitable environment in which to perform trait
    /// queries on the given value. When type-checking, this is simply
    /// the pair of the environment plus value. But when reveal is set to
    /// All, then if `value` does not reference any type parameters, we will
    /// pair it with the empty environment. This improves caching and is generally
    /// invisible.
    ///
    /// NB: We preserve the environment when type-checking because it
    /// is possible for the user to have wacky where-clauses like
    /// `where Box<u32>: Copy`, which are clearly never
    /// satisfiable. We generally want to behave as if they were true,
    /// although the surrounding function is never reachable.
    pub fn and<T: TypeFoldable<'tcx>>(self, value: T) -> ParamEnvAnd<'tcx, T> {
        match self.reveal {
            Reveal::UserFacing => {
                ParamEnvAnd {
                    param_env: self,
                    value,
                }
            }

            Reveal::All => {
                if value.has_skol()
                    || value.needs_infer()
                    || value.has_param_types()
                    || value.has_self_ty()
                {
                    ParamEnvAnd {
                        param_env: self,
                        value,
                    }
                } else {
                    ParamEnvAnd {
                        param_env: self.without_caller_bounds(),
                        value,
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamEnvAnd<'tcx, T> {
    pub param_env: ParamEnv<'tcx>,
    pub value: T,
}

impl<'tcx, T> ParamEnvAnd<'tcx, T> {
    pub fn into_parts(self) -> (ParamEnv<'tcx>, T) {
        (self.param_env, self.value)
    }
}

impl<'a, 'gcx, T> HashStable<StableHashingContext<'a>> for ParamEnvAnd<'gcx, T>
    where T: HashStable<StableHashingContext<'a>>
{
    fn hash_stable<W: StableHasherResult>(&self,
                                          hcx: &mut StableHashingContext<'a>,
                                          hasher: &mut StableHasher<W>) {
        let ParamEnvAnd {
            ref param_env,
            ref value
        } = *self;

        param_env.hash_stable(hcx, hasher);
        value.hash_stable(hcx, hasher);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Destructor {
    /// The def-id of the destructor method
    pub did: DefId,
}

bitflags! {
    pub struct AdtFlags: u32 {
        const NO_ADT_FLAGS        = 0;
        const IS_ENUM             = 1 << 0;
        const IS_PHANTOM_DATA     = 1 << 1;
        const IS_FUNDAMENTAL      = 1 << 2;
        const IS_UNION            = 1 << 3;
        const IS_BOX              = 1 << 4;
        /// Indicates whether this abstract data type will be expanded on in future (new
        /// fields/variants) and as such, whether downstream crates must match exhaustively on the
        /// fields/variants of this data type.
        ///
        /// See RFC 2008 (<https://github.com/rust-lang/rfcs/pull/2008>).
        const IS_NON_EXHAUSTIVE   = 1 << 5;
    }
}

#[derive(Debug)]
pub struct VariantDef {
    /// The variant's DefId. If this is a tuple-like struct,
    /// this is the DefId of the struct's ctor.
    pub did: DefId,
    pub name: Name, // struct's name if this is a struct
    pub discr: VariantDiscr,
    pub fields: Vec<FieldDef>,
    pub ctor_kind: CtorKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, RustcEncodable, RustcDecodable)]
pub enum VariantDiscr {
    /// Explicit value for this variant, i.e. `X = 123`.
    /// The `DefId` corresponds to the embedded constant.
    Explicit(DefId),

    /// The previous variant's discriminant plus one.
    /// For efficiency reasons, the distance from the
    /// last `Explicit` discriminant is being stored,
    /// or `0` for the first variant, if it has none.
    Relative(usize),
}

#[derive(Debug)]
pub struct FieldDef {
    pub did: DefId,
    pub ident: Ident,
    pub vis: Visibility,
}

/// The definition of an abstract data type - a struct or enum.
///
/// These are all interned (by intern_adt_def) into the adt_defs
/// table.
pub struct AdtDef {
    pub did: DefId,
    pub variants: Vec<VariantDef>,
    flags: AdtFlags,
    pub repr: ReprOptions,
}

impl PartialOrd for AdtDef {
    fn partial_cmp(&self, other: &AdtDef) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

/// There should be only one AdtDef for each `did`, therefore
/// it is fine to implement `Ord` only based on `did`.
impl Ord for AdtDef {
    fn cmp(&self, other: &AdtDef) -> Ordering {
        self.did.cmp(&other.did)
    }
}

impl PartialEq for AdtDef {
    // AdtDef are always interned and this is part of TyS equality
    #[inline]
    fn eq(&self, other: &Self) -> bool { self as *const _ == other as *const _ }
}

impl Eq for AdtDef {}

impl Hash for AdtDef {
    #[inline]
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self as *const AdtDef).hash(s)
    }
}

impl<'tcx> serialize::UseSpecializedEncodable for &'tcx AdtDef {
    fn default_encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.did.encode(s)
    }
}

impl<'tcx> serialize::UseSpecializedDecodable for &'tcx AdtDef {}


impl<'a> HashStable<StableHashingContext<'a>> for AdtDef {
    fn hash_stable<W: StableHasherResult>(&self,
                                          hcx: &mut StableHashingContext<'a>,
                                          hasher: &mut StableHasher<W>) {
        thread_local! {
            static CACHE: RefCell<FxHashMap<usize, Fingerprint>> =
                RefCell::new(FxHashMap());
        }

        let hash: Fingerprint = CACHE.with(|cache| {
            let addr = self as *const AdtDef as usize;
            *cache.borrow_mut().entry(addr).or_insert_with(|| {
                let ty::AdtDef {
                    did,
                    ref variants,
                    ref flags,
                    ref repr,
                } = *self;

                let mut hasher = StableHasher::new();
                did.hash_stable(hcx, &mut hasher);
                variants.hash_stable(hcx, &mut hasher);
                flags.hash_stable(hcx, &mut hasher);
                repr.hash_stable(hcx, &mut hasher);

                hasher.finish()
           })
        });

        hash.hash_stable(hcx, hasher);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum AdtKind { Struct, Union, Enum }

bitflags! {
    #[derive(RustcEncodable, RustcDecodable, Default)]
    pub struct ReprFlags: u8 {
        const IS_C               = 1 << 0;
        const IS_SIMD            = 1 << 1;
        const IS_TRANSPARENT     = 1 << 2;
        // Internal only for now. If true, don't reorder fields.
        const IS_LINEAR          = 1 << 3;

        // Any of these flags being set prevent field reordering optimisation.
        const IS_UNOPTIMISABLE   = ReprFlags::IS_C.bits |
                                   ReprFlags::IS_SIMD.bits |
                                   ReprFlags::IS_LINEAR.bits;
    }
}

impl_stable_hash_for!(struct ReprFlags {
    bits
});



/// Represents the repr options provided by the user,
#[derive(Copy, Clone, Eq, PartialEq, RustcEncodable, RustcDecodable, Default)]
pub struct ReprOptions {
    pub int: Option<attr::IntType>,
    pub align: u32,
    pub pack: u32,
    pub flags: ReprFlags,
}

impl_stable_hash_for!(struct ReprOptions {
    align,
    pack,
    int,
    flags
});

impl ReprOptions {
    pub fn new(tcx: TyCtxt, did: DefId) -> ReprOptions {
        let mut flags = ReprFlags::empty();
        let mut size = None;
        let mut max_align = 0;
        let mut min_pack = 0;
        for attr in tcx.get_attrs(did).iter() {
            for r in attr::find_repr_attrs(tcx.sess.diagnostic(), attr) {
                flags.insert(match r {
                    attr::ReprC => ReprFlags::IS_C,
                    attr::ReprPacked(pack) => {
                        min_pack = if min_pack > 0 {
                            cmp::min(pack, min_pack)
                        } else {
                            pack
                        };
                        ReprFlags::empty()
                    },
                    attr::ReprTransparent => ReprFlags::IS_TRANSPARENT,
                    attr::ReprSimd => ReprFlags::IS_SIMD,
                    attr::ReprInt(i) => {
                        size = Some(i);
                        ReprFlags::empty()
                    },
                    attr::ReprAlign(align) => {
                        max_align = cmp::max(align, max_align);
                        ReprFlags::empty()
                    },
                });
            }
        }

        // This is here instead of layout because the choice must make it into metadata.
        if !tcx.consider_optimizing(|| format!("Reorder fields of {:?}", tcx.item_path_str(did))) {
            flags.insert(ReprFlags::IS_LINEAR);
        }
        ReprOptions { int: size, align: max_align, pack: min_pack, flags: flags }
    }

    #[inline]
    pub fn simd(&self) -> bool { self.flags.contains(ReprFlags::IS_SIMD) }
    #[inline]
    pub fn c(&self) -> bool { self.flags.contains(ReprFlags::IS_C) }
    #[inline]
    pub fn packed(&self) -> bool { self.pack > 0 }
    #[inline]
    pub fn transparent(&self) -> bool { self.flags.contains(ReprFlags::IS_TRANSPARENT) }
    #[inline]
    pub fn linear(&self) -> bool { self.flags.contains(ReprFlags::IS_LINEAR) }

    pub fn discr_type(&self) -> attr::IntType {
        self.int.unwrap_or(attr::SignedInt(ast::IntTy::Isize))
    }

    /// Returns true if this `#[repr()]` should inhabit "smart enum
    /// layout" optimizations, such as representing `Foo<&T>` as a
    /// single pointer.
    pub fn inhibit_enum_layout_opt(&self) -> bool {
        self.c() || self.int.is_some()
    }

    /// Returns true if this `#[repr()]` should inhibit struct field reordering
    /// optimizations, such as with repr(C) or repr(packed(1)).
    pub fn inhibit_struct_field_reordering_opt(&self) -> bool {
        !(self.flags & ReprFlags::IS_UNOPTIMISABLE).is_empty() || (self.pack == 1)
    }
}

impl<'a, 'gcx, 'tcx> AdtDef {
    fn new(tcx: TyCtxt,
           did: DefId,
           kind: AdtKind,
           variants: Vec<VariantDef>,
           repr: ReprOptions) -> Self {
        let mut flags = AdtFlags::NO_ADT_FLAGS;
        let attrs = tcx.get_attrs(did);
        if attr::contains_name(&attrs, "fundamental") {
            flags = flags | AdtFlags::IS_FUNDAMENTAL;
        }
        if Some(did) == tcx.lang_items().phantom_data() {
            flags = flags | AdtFlags::IS_PHANTOM_DATA;
        }
        if Some(did) == tcx.lang_items().owned_box() {
            flags = flags | AdtFlags::IS_BOX;
        }
        if tcx.has_attr(did, "non_exhaustive") {
            flags = flags | AdtFlags::IS_NON_EXHAUSTIVE;
        }
        match kind {
            AdtKind::Enum => flags = flags | AdtFlags::IS_ENUM,
            AdtKind::Union => flags = flags | AdtFlags::IS_UNION,
            AdtKind::Struct => {}
        }
        AdtDef {
            did,
            variants,
            flags,
            repr,
        }
    }

    #[inline]
    pub fn is_struct(&self) -> bool {
        !self.is_union() && !self.is_enum()
    }

    #[inline]
    pub fn is_union(&self) -> bool {
        self.flags.intersects(AdtFlags::IS_UNION)
    }

    #[inline]
    pub fn is_enum(&self) -> bool {
        self.flags.intersects(AdtFlags::IS_ENUM)
    }

    #[inline]
    pub fn is_non_exhaustive(&self) -> bool {
        self.flags.intersects(AdtFlags::IS_NON_EXHAUSTIVE)
    }

    /// Returns the kind of the ADT - Struct or Enum.
    #[inline]
    pub fn adt_kind(&self) -> AdtKind {
        if self.is_enum() {
            AdtKind::Enum
        } else if self.is_union() {
            AdtKind::Union
        } else {
            AdtKind::Struct
        }
    }

    pub fn descr(&self) -> &'static str {
        match self.adt_kind() {
            AdtKind::Struct => "struct",
            AdtKind::Union => "union",
            AdtKind::Enum => "enum",
        }
    }

    pub fn variant_descr(&self) -> &'static str {
        match self.adt_kind() {
            AdtKind::Struct => "struct",
            AdtKind::Union => "union",
            AdtKind::Enum => "variant",
        }
    }

    /// Returns whether this type is #[fundamental] for the purposes
    /// of coherence checking.
    #[inline]
    pub fn is_fundamental(&self) -> bool {
        self.flags.intersects(AdtFlags::IS_FUNDAMENTAL)
    }

    /// Returns true if this is PhantomData<T>.
    #[inline]
    pub fn is_phantom_data(&self) -> bool {
        self.flags.intersects(AdtFlags::IS_PHANTOM_DATA)
    }

    /// Returns true if this is Box<T>.
    #[inline]
    pub fn is_box(&self) -> bool {
        self.flags.intersects(AdtFlags::IS_BOX)
    }

    /// Returns whether this type has a destructor.
    pub fn has_dtor(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> bool {
        self.destructor(tcx).is_some()
    }

    /// Asserts this is a struct or union and returns its unique variant.
    pub fn non_enum_variant(&self) -> &VariantDef {
        assert!(self.is_struct() || self.is_union());
        &self.variants[0]
    }

    #[inline]
    pub fn predicates(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> GenericPredicates<'gcx> {
        tcx.predicates_of(self.did)
    }

    /// Returns an iterator over all fields contained
    /// by this ADT.
    #[inline]
    pub fn all_fields<'s>(&'s self) -> impl Iterator<Item = &'s FieldDef> {
        self.variants.iter().flat_map(|v| v.fields.iter())
    }

    pub fn is_payloadfree(&self) -> bool {
        !self.variants.is_empty() &&
            self.variants.iter().all(|v| v.fields.is_empty())
    }

    pub fn variant_with_id(&self, vid: DefId) -> &VariantDef {
        self.variants
            .iter()
            .find(|v| v.did == vid)
            .expect("variant_with_id: unknown variant")
    }

    pub fn variant_index_with_id(&self, vid: DefId) -> usize {
        self.variants
            .iter()
            .position(|v| v.did == vid)
            .expect("variant_index_with_id: unknown variant")
    }

    pub fn variant_of_def(&self, def: Def) -> &VariantDef {
        match def {
            Def::Variant(vid) | Def::VariantCtor(vid, ..) => self.variant_with_id(vid),
            Def::Struct(..) | Def::StructCtor(..) | Def::Union(..) |
            Def::TyAlias(..) | Def::AssociatedTy(..) | Def::SelfTy(..) => self.non_enum_variant(),
            _ => bug!("unexpected def {:?} in variant_of_def", def)
        }
    }

    #[inline]
    pub fn eval_explicit_discr(
        &self,
        tcx: TyCtxt<'a, 'gcx, 'tcx>,
        expr_did: DefId,
    ) -> Option<Discr<'tcx>> {
        let param_env = ParamEnv::empty();
        let repr_type = self.repr.discr_type();
        let substs = Substs::identity_for_item(tcx.global_tcx(), expr_did);
        let instance = ty::Instance::new(expr_did, substs);
        let cid = GlobalId {
            instance,
            promoted: None
        };
        match tcx.const_eval(param_env.and(cid)) {
            Ok(val) => {
                // FIXME: Find the right type and use it instead of `val.ty` here
                if let Some(b) = val.assert_bits(tcx.global_tcx(), param_env.and(val.ty)) {
                    trace!("discriminants: {} ({:?})", b, repr_type);
                    Some(Discr {
                        val: b,
                        ty: val.ty,
                    })
                } else {
                    info!("invalid enum discriminant: {:#?}", val);
                    ::middle::const_val::struct_error(
                        tcx.at(tcx.def_span(expr_did)),
                        "constant evaluation of enum discriminant resulted in non-integer",
                    ).emit();
                    None
                }
            }
            Err(err) => {
                err.report_as_error(
                    tcx.at(tcx.def_span(expr_did)),
                    "could not evaluate enum discriminant",
                );
                if !expr_did.is_local() {
                    span_bug!(tcx.def_span(expr_did),
                        "variant discriminant evaluation succeeded \
                            in its crate but failed locally");
                }
                None
            }
        }
    }

    #[inline]
    pub fn discriminants(
        &'a self,
        tcx: TyCtxt<'a, 'gcx, 'tcx>,
    ) -> impl Iterator<Item=Discr<'tcx>> + Captures<'gcx> + 'a {
        let repr_type = self.repr.discr_type();
        let initial = repr_type.initial_discriminant(tcx.global_tcx());
        let mut prev_discr = None::<Discr<'tcx>>;
        self.variants.iter().map(move |v| {
            let mut discr = prev_discr.map_or(initial, |d| d.wrap_incr(tcx));
            if let VariantDiscr::Explicit(expr_did) = v.discr {
                if let Some(new_discr) = self.eval_explicit_discr(tcx, expr_did) {
                    discr = new_discr;
                }
            }
            prev_discr = Some(discr);

            discr
        })
    }

    /// Compute the discriminant value used by a specific variant.
    /// Unlike `discriminants`, this is (amortized) constant-time,
    /// only doing at most one query for evaluating an explicit
    /// discriminant (the last one before the requested variant),
    /// assuming there are no constant-evaluation errors there.
    pub fn discriminant_for_variant(&self,
                                    tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                    variant_index: usize)
                                    -> Discr<'tcx> {
        let (val, offset) = self.discriminant_def_for_variant(variant_index);
        let explicit_value = val
            .and_then(|expr_did| self.eval_explicit_discr(tcx, expr_did))
            .unwrap_or_else(|| self.repr.discr_type().initial_discriminant(tcx.global_tcx()));
        explicit_value.checked_add(tcx, offset as u128).0
    }

    /// Yields a DefId for the discriminant and an offset to add to it
    /// Alternatively, if there is no explicit discriminant, returns the
    /// inferred discriminant directly
    pub fn discriminant_def_for_variant(
        &self,
        variant_index: usize,
    ) -> (Option<DefId>, usize) {
        let mut explicit_index = variant_index;
        let expr_did;
        loop {
            match self.variants[explicit_index].discr {
                ty::VariantDiscr::Relative(0) => {
                    expr_did = None;
                    break;
                },
                ty::VariantDiscr::Relative(distance) => {
                    explicit_index -= distance;
                }
                ty::VariantDiscr::Explicit(did) => {
                    expr_did = Some(did);
                    break;
                }
            }
        }
        (expr_did, variant_index - explicit_index)
    }

    pub fn destructor(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Option<Destructor> {
        tcx.adt_destructor(self.did)
    }

    /// Returns a list of types such that `Self: Sized` if and only
    /// if that type is Sized, or `TyErr` if this type is recursive.
    ///
    /// Oddly enough, checking that the sized-constraint is Sized is
    /// actually more expressive than checking all members:
    /// the Sized trait is inductive, so an associated type that references
    /// Self would prevent its containing ADT from being Sized.
    ///
    /// Due to normalization being eager, this applies even if
    /// the associated type is behind a pointer, e.g. issue #31299.
    pub fn sized_constraint(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> &'tcx [Ty<'tcx>] {
        match tcx.try_adt_sized_constraint(DUMMY_SP, self.did) {
            Ok(tys) => tys,
            Err(mut bug) => {
                debug!("adt_sized_constraint: {:?} is recursive", self);
                // This should be reported as an error by `check_representable`.
                //
                // Consider the type as Sized in the meanwhile to avoid
                // further errors. Delay our `bug` diagnostic here to get
                // emitted later as well in case we accidentally otherwise don't
                // emit an error.
                bug.delay_as_bug();
                tcx.intern_type_list(&[tcx.types.err])
            }
        }
    }

    fn sized_constraint_for_ty(&self,
                               tcx: TyCtxt<'a, 'tcx, 'tcx>,
                               ty: Ty<'tcx>)
                               -> Vec<Ty<'tcx>> {
        let result = match ty.sty {
            TyBool | TyChar | TyInt(..) | TyUint(..) | TyFloat(..) |
            TyRawPtr(..) | TyRef(..) | TyFnDef(..) | TyFnPtr(_) |
            TyArray(..) | TyClosure(..) | TyGenerator(..) | TyNever => {
                vec![]
            }

            TyStr |
            TyDynamic(..) |
            TySlice(_) |
            TyForeign(..) |
            TyError |
            TyGeneratorWitness(..) => {
                // these are never sized - return the target type
                vec![ty]
            }

            TyTuple(ref tys) => {
                match tys.last() {
                    None => vec![],
                    Some(ty) => self.sized_constraint_for_ty(tcx, ty)
                }
            }

            TyAdt(adt, substs) => {
                // recursive case
                let adt_tys = adt.sized_constraint(tcx);
                debug!("sized_constraint_for_ty({:?}) intermediate = {:?}",
                       ty, adt_tys);
                adt_tys.iter()
                    .map(|ty| ty.subst(tcx, substs))
                    .flat_map(|ty| self.sized_constraint_for_ty(tcx, ty))
                    .collect()
            }

            TyProjection(..) | TyAnon(..) => {
                // must calculate explicitly.
                // FIXME: consider special-casing always-Sized projections
                vec![ty]
            }

            TyParam(..) => {
                // perf hack: if there is a `T: Sized` bound, then
                // we know that `T` is Sized and do not need to check
                // it on the impl.

                let sized_trait = match tcx.lang_items().sized_trait() {
                    Some(x) => x,
                    _ => return vec![ty]
                };
                let sized_predicate = Binder::dummy(TraitRef {
                    def_id: sized_trait,
                    substs: tcx.mk_substs_trait(ty, &[])
                }).to_predicate();
                let predicates = tcx.predicates_of(self.did).predicates;
                if predicates.into_iter().any(|p| p == sized_predicate) {
                    vec![]
                } else {
                    vec![ty]
                }
            }

            TyInfer(..) => {
                bug!("unexpected type `{:?}` in sized_constraint_for_ty",
                     ty)
            }
        };
        debug!("sized_constraint_for_ty({:?}) = {:?}", ty, result);
        result
    }
}

impl<'a, 'gcx, 'tcx> FieldDef {
    pub fn ty(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, subst: &Substs<'tcx>) -> Ty<'tcx> {
        tcx.type_of(self.did).subst(tcx, subst)
    }
}

/// Represents the various closure traits in the Rust language. This
/// will determine the type of the environment (`self`, in the
/// desuaring) argument that the closure expects.
///
/// You can get the environment type of a closure using
/// `tcx.closure_env_ty()`.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable)]
pub enum ClosureKind {
    // Warning: Ordering is significant here! The ordering is chosen
    // because the trait Fn is a subtrait of FnMut and so in turn, and
    // hence we order it so that Fn < FnMut < FnOnce.
    Fn,
    FnMut,
    FnOnce,
}

impl<'a, 'tcx> ClosureKind {
    // This is the initial value used when doing upvar inference.
    pub const LATTICE_BOTTOM: ClosureKind = ClosureKind::Fn;

    pub fn trait_did(&self, tcx: TyCtxt<'a, 'tcx, 'tcx>) -> DefId {
        match *self {
            ClosureKind::Fn => tcx.require_lang_item(FnTraitLangItem),
            ClosureKind::FnMut => {
                tcx.require_lang_item(FnMutTraitLangItem)
            }
            ClosureKind::FnOnce => {
                tcx.require_lang_item(FnOnceTraitLangItem)
            }
        }
    }

    /// True if this a type that impls this closure kind
    /// must also implement `other`.
    pub fn extends(self, other: ty::ClosureKind) -> bool {
        match (self, other) {
            (ClosureKind::Fn, ClosureKind::Fn) => true,
            (ClosureKind::Fn, ClosureKind::FnMut) => true,
            (ClosureKind::Fn, ClosureKind::FnOnce) => true,
            (ClosureKind::FnMut, ClosureKind::FnMut) => true,
            (ClosureKind::FnMut, ClosureKind::FnOnce) => true,
            (ClosureKind::FnOnce, ClosureKind::FnOnce) => true,
            _ => false,
        }
    }

    /// Returns the representative scalar type for this closure kind.
    /// See `TyS::to_opt_closure_kind` for more details.
    pub fn to_ty(self, tcx: TyCtxt<'_, '_, 'tcx>) -> Ty<'tcx> {
        match self {
            ty::ClosureKind::Fn => tcx.types.i8,
            ty::ClosureKind::FnMut => tcx.types.i16,
            ty::ClosureKind::FnOnce => tcx.types.i32,
        }
    }
}

impl<'tcx> TyS<'tcx> {
    /// Iterator that walks `self` and any types reachable from
    /// `self`, in depth-first order. Note that just walks the types
    /// that appear in `self`, it does not descend into the fields of
    /// structs or variants. For example:
    ///
    /// ```notrust
    /// isize => { isize }
    /// Foo<Bar<isize>> => { Foo<Bar<isize>>, Bar<isize>, isize }
    /// [isize] => { [isize], isize }
    /// ```
    pub fn walk(&'tcx self) -> TypeWalker<'tcx> {
        TypeWalker::new(self)
    }

    /// Iterator that walks the immediate children of `self`.  Hence
    /// `Foo<Bar<i32>, u32>` yields the sequence `[Bar<i32>, u32]`
    /// (but not `i32`, like `walk`).
    pub fn walk_shallow(&'tcx self) -> AccIntoIter<walk::TypeWalkerArray<'tcx>> {
        walk::walk_shallow(self)
    }

    /// Walks `ty` and any types appearing within `ty`, invoking the
    /// callback `f` on each type. If the callback returns false, then the
    /// children of the current type are ignored.
    ///
    /// Note: prefer `ty.walk()` where possible.
    pub fn maybe_walk<F>(&'tcx self, mut f: F)
        where F : FnMut(Ty<'tcx>) -> bool
    {
        let mut walker = self.walk();
        while let Some(ty) = walker.next() {
            if !f(ty) {
                walker.skip_current_subtree();
            }
        }
    }
}

impl BorrowKind {
    pub fn from_mutbl(m: hir::Mutability) -> BorrowKind {
        match m {
            hir::MutMutable => MutBorrow,
            hir::MutImmutable => ImmBorrow,
        }
    }

    /// Returns a mutability `m` such that an `&m T` pointer could be used to obtain this borrow
    /// kind. Because borrow kinds are richer than mutabilities, we sometimes have to pick a
    /// mutability that is stronger than necessary so that it at least *would permit* the borrow in
    /// question.
    pub fn to_mutbl_lossy(self) -> hir::Mutability {
        match self {
            MutBorrow => hir::MutMutable,
            ImmBorrow => hir::MutImmutable,

            // We have no type corresponding to a unique imm borrow, so
            // use `&mut`. It gives all the capabilities of an `&uniq`
            // and hence is a safe "over approximation".
            UniqueImmBorrow => hir::MutMutable,
        }
    }

    pub fn to_user_str(&self) -> &'static str {
        match *self {
            MutBorrow => "mutable",
            ImmBorrow => "immutable",
            UniqueImmBorrow => "uniquely immutable",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Attributes<'gcx> {
    Owned(Lrc<[ast::Attribute]>),
    Borrowed(&'gcx [ast::Attribute])
}

impl<'gcx> ::std::ops::Deref for Attributes<'gcx> {
    type Target = [ast::Attribute];

    fn deref(&self) -> &[ast::Attribute] {
        match self {
            &Attributes::Owned(ref data) => &data,
            &Attributes::Borrowed(data) => data
        }
    }
}

impl<'a, 'gcx, 'tcx> TyCtxt<'a, 'gcx, 'tcx> {
    pub fn body_tables(self, body: hir::BodyId) -> &'gcx TypeckTables<'gcx> {
        self.typeck_tables_of(self.hir.body_owner_def_id(body))
    }

    /// Returns an iterator of the def-ids for all body-owners in this
    /// crate. If you would prefer to iterate over the bodies
    /// themselves, you can do `self.hir.krate().body_ids.iter()`.
    pub fn body_owners(
        self,
    ) -> impl Iterator<Item = DefId> + Captures<'tcx> + Captures<'gcx> + 'a {
        self.hir.krate()
                .body_ids
                .iter()
                .map(move |&body_id| self.hir.body_owner_def_id(body_id))
    }

    pub fn par_body_owners<F: Fn(DefId) + sync::Sync + sync::Send>(self, f: F) {
        par_iter(&self.hir.krate().body_ids).for_each(|&body_id| {
            f(self.hir.body_owner_def_id(body_id))
        });
    }

    pub fn expr_span(self, id: NodeId) -> Span {
        match self.hir.find(id) {
            Some(hir_map::NodeExpr(e)) => {
                e.span
            }
            Some(f) => {
                bug!("Node id {} is not an expr: {:?}", id, f);
            }
            None => {
                bug!("Node id {} is not present in the node map", id);
            }
        }
    }

    pub fn provided_trait_methods(self, id: DefId) -> Vec<AssociatedItem> {
        self.associated_items(id)
            .filter(|item| item.kind == AssociatedKind::Method && item.defaultness.has_value())
            .collect()
    }

    pub fn trait_relevant_for_never(self, did: DefId) -> bool {
        self.associated_items(did).any(|item| {
            item.relevant_for_never()
        })
    }

    pub fn opt_associated_item(self, def_id: DefId) -> Option<AssociatedItem> {
        let is_associated_item = if let Some(node_id) = self.hir.as_local_node_id(def_id) {
            match self.hir.get(node_id) {
                hir_map::NodeTraitItem(_) | hir_map::NodeImplItem(_) => true,
                _ => false,
            }
        } else {
            match self.describe_def(def_id).expect("no def for def-id") {
                Def::AssociatedConst(_) | Def::Method(_) | Def::AssociatedTy(_) => true,
                _ => false,
            }
        };

        if is_associated_item {
            Some(self.associated_item(def_id))
        } else {
            None
        }
    }

    fn associated_item_from_trait_item_ref(self,
                                           parent_def_id: DefId,
                                           parent_vis: &hir::Visibility,
                                           trait_item_ref: &hir::TraitItemRef)
                                           -> AssociatedItem {
        let def_id = self.hir.local_def_id(trait_item_ref.id.node_id);
        let (kind, has_self) = match trait_item_ref.kind {
            hir::AssociatedItemKind::Const => (ty::AssociatedKind::Const, false),
            hir::AssociatedItemKind::Method { has_self } => {
                (ty::AssociatedKind::Method, has_self)
            }
            hir::AssociatedItemKind::Type => (ty::AssociatedKind::Type, false),
        };

        AssociatedItem {
            name: trait_item_ref.name,
            kind,
            // Visibility of trait items is inherited from their traits.
            vis: Visibility::from_hir(parent_vis, trait_item_ref.id.node_id, self),
            defaultness: trait_item_ref.defaultness,
            def_id,
            container: TraitContainer(parent_def_id),
            method_has_self_argument: has_self
        }
    }

    fn associated_item_from_impl_item_ref(self,
                                          parent_def_id: DefId,
                                          impl_item_ref: &hir::ImplItemRef)
                                          -> AssociatedItem {
        let def_id = self.hir.local_def_id(impl_item_ref.id.node_id);
        let (kind, has_self) = match impl_item_ref.kind {
            hir::AssociatedItemKind::Const => (ty::AssociatedKind::Const, false),
            hir::AssociatedItemKind::Method { has_self } => {
                (ty::AssociatedKind::Method, has_self)
            }
            hir::AssociatedItemKind::Type => (ty::AssociatedKind::Type, false),
        };

        ty::AssociatedItem {
            name: impl_item_ref.name,
            kind,
            // Visibility of trait impl items doesn't matter.
            vis: ty::Visibility::from_hir(&impl_item_ref.vis, impl_item_ref.id.node_id, self),
            defaultness: impl_item_ref.defaultness,
            def_id,
            container: ImplContainer(parent_def_id),
            method_has_self_argument: has_self
        }
    }

    pub fn field_index(self, node_id: NodeId, tables: &TypeckTables) -> usize {
        let hir_id = self.hir.node_to_hir_id(node_id);
        tables.field_indices().get(hir_id).cloned().expect("no index for a field")
    }

    pub fn find_field_index(self, ident: Ident, variant: &VariantDef) -> Option<usize> {
        variant.fields.iter().position(|field| {
            self.adjust_ident(ident, variant.did, DUMMY_NODE_ID).0 == field.ident.modern()
        })
    }

    pub fn associated_items(
        self,
        def_id: DefId,
    ) -> impl Iterator<Item = ty::AssociatedItem> + 'a {
        let def_ids = self.associated_item_def_ids(def_id);
        Box::new((0..def_ids.len()).map(move |i| self.associated_item(def_ids[i])))
            as Box<dyn Iterator<Item = ty::AssociatedItem> + 'a>
    }

    /// Returns true if the impls are the same polarity and are implementing
    /// a trait which contains no items
    pub fn impls_are_allowed_to_overlap(self, def_id1: DefId, def_id2: DefId) -> bool {
        if !self.features().overlapping_marker_traits {
            return false;
        }
        let trait1_is_empty = self.impl_trait_ref(def_id1)
            .map_or(false, |trait_ref| {
                self.associated_item_def_ids(trait_ref.def_id).is_empty()
            });
        let trait2_is_empty = self.impl_trait_ref(def_id2)
            .map_or(false, |trait_ref| {
                self.associated_item_def_ids(trait_ref.def_id).is_empty()
            });
        self.impl_polarity(def_id1) == self.impl_polarity(def_id2)
            && trait1_is_empty
            && trait2_is_empty
    }

    // Returns `ty::VariantDef` if `def` refers to a struct,
    // or variant or their constructors, panics otherwise.
    pub fn expect_variant_def(self, def: Def) -> &'tcx VariantDef {
        match def {
            Def::Variant(did) | Def::VariantCtor(did, ..) => {
                let enum_did = self.parent_def_id(did).unwrap();
                self.adt_def(enum_did).variant_with_id(did)
            }
            Def::Struct(did) | Def::Union(did) => {
                self.adt_def(did).non_enum_variant()
            }
            Def::StructCtor(ctor_did, ..) => {
                let did = self.parent_def_id(ctor_did).expect("struct ctor has no parent");
                self.adt_def(did).non_enum_variant()
            }
            _ => bug!("expect_variant_def used with unexpected def {:?}", def)
        }
    }

    /// Given a `VariantDef`, returns the def-id of the `AdtDef` of which it is a part.
    pub fn adt_def_id_of_variant(self, variant_def: &'tcx VariantDef) -> DefId {
        let def_key = self.def_key(variant_def.did);
        match def_key.disambiguated_data.data {
            // for enum variants and tuple structs, the def-id of the ADT itself
            // is the *parent* of the variant
            DefPathData::EnumVariant(..) | DefPathData::StructCtor =>
                DefId { krate: variant_def.did.krate, index: def_key.parent.unwrap() },

            // otherwise, for structs and unions, they share a def-id
            _ => variant_def.did,
        }
    }

    pub fn item_name(self, id: DefId) -> InternedString {
        if id.index == CRATE_DEF_INDEX {
            self.original_crate_name(id.krate).as_interned_str()
        } else {
            let def_key = self.def_key(id);
            // The name of a StructCtor is that of its struct parent.
            if let hir_map::DefPathData::StructCtor = def_key.disambiguated_data.data {
                self.item_name(DefId {
                    krate: id.krate,
                    index: def_key.parent.unwrap()
                })
            } else {
                def_key.disambiguated_data.data.get_opt_name().unwrap_or_else(|| {
                    bug!("item_name: no name for {:?}", self.def_path(id));
                })
            }
        }
    }

    /// Return the possibly-auto-generated MIR of a (DefId, Subst) pair.
    pub fn instance_mir(self, instance: ty::InstanceDef<'gcx>)
                        -> &'gcx Mir<'gcx>
    {
        match instance {
            ty::InstanceDef::Item(did) => {
                self.optimized_mir(did)
            }
            ty::InstanceDef::Intrinsic(..) |
            ty::InstanceDef::FnPtrShim(..) |
            ty::InstanceDef::Virtual(..) |
            ty::InstanceDef::ClosureOnceShim { .. } |
            ty::InstanceDef::DropGlue(..) |
            ty::InstanceDef::CloneShim(..) => {
                self.mir_shims(instance)
            }
        }
    }

    /// Given the DefId of an item, returns its MIR, borrowed immutably.
    /// Returns None if there is no MIR for the DefId
    pub fn maybe_optimized_mir(self, did: DefId) -> Option<&'gcx Mir<'gcx>> {
        if self.is_mir_available(did) {
            Some(self.optimized_mir(did))
        } else {
            None
        }
    }

    /// Get the attributes of a definition.
    pub fn get_attrs(self, did: DefId) -> Attributes<'gcx> {
        if let Some(id) = self.hir.as_local_node_id(did) {
            Attributes::Borrowed(self.hir.attrs(id))
        } else {
            Attributes::Owned(self.item_attrs(did))
        }
    }

    /// Determine whether an item is annotated with an attribute
    pub fn has_attr(self, did: DefId, attr: &str) -> bool {
        attr::contains_name(&self.get_attrs(did), attr)
    }

    /// Returns true if this is an `auto trait`.
    pub fn trait_is_auto(self, trait_def_id: DefId) -> bool {
        self.trait_def(trait_def_id).has_auto_impl
    }

    pub fn generator_layout(self, def_id: DefId) -> &'tcx GeneratorLayout<'tcx> {
        self.optimized_mir(def_id).generator_layout.as_ref().unwrap()
    }

    /// Given the def_id of an impl, return the def_id of the trait it implements.
    /// If it implements no trait, return `None`.
    pub fn trait_id_of_impl(self, def_id: DefId) -> Option<DefId> {
        self.impl_trait_ref(def_id).map(|tr| tr.def_id)
    }

    /// If the given def ID describes a method belonging to an impl, return the
    /// ID of the impl that the method belongs to. Otherwise, return `None`.
    pub fn impl_of_method(self, def_id: DefId) -> Option<DefId> {
        let item = if def_id.krate != LOCAL_CRATE {
            if let Some(Def::Method(_)) = self.describe_def(def_id) {
                Some(self.associated_item(def_id))
            } else {
                None
            }
        } else {
            self.opt_associated_item(def_id)
        };

        match item {
            Some(trait_item) => {
                match trait_item.container {
                    TraitContainer(_) => None,
                    ImplContainer(def_id) => Some(def_id),
                }
            }
            None => None
        }
    }

    /// Looks up the span of `impl_did` if the impl is local; otherwise returns `Err`
    /// with the name of the crate containing the impl.
    pub fn span_of_impl(self, impl_did: DefId) -> Result<Span, Symbol> {
        if impl_did.is_local() {
            let node_id = self.hir.as_local_node_id(impl_did).unwrap();
            Ok(self.hir.span(node_id))
        } else {
            Err(self.crate_name(impl_did.krate))
        }
    }

    // Hygienically compare a use-site name (`use_name`) for a field or an associated item with its
    // supposed definition name (`def_name`). The method also needs `DefId` of the supposed
    // definition's parent/scope to perform comparison.
    pub fn hygienic_eq(self, use_name: Name, def_name: Name, def_parent_def_id: DefId) -> bool {
        let (use_ident, def_ident) = (use_name.to_ident(), def_name.to_ident());
        self.adjust_ident(use_ident, def_parent_def_id, DUMMY_NODE_ID).0 == def_ident
    }

    pub fn adjust_ident(self, mut ident: Ident, scope: DefId, block: NodeId) -> (Ident, DefId) {
        let expansion = match scope.krate {
            LOCAL_CRATE => self.hir.definitions().expansion(scope.index),
            _ => Mark::root(),
        };
        ident = ident.modern();
        let scope = match ident.span.adjust(expansion) {
            Some(macro_def) => self.hir.definitions().macro_def_scope(macro_def),
            None if block == DUMMY_NODE_ID => DefId::local(CRATE_DEF_INDEX), // Dummy DefId
            None => self.hir.get_module_parent(block),
        };
        (ident, scope)
    }
}

impl<'a, 'gcx, 'tcx> TyCtxt<'a, 'gcx, 'tcx> {
    pub fn with_freevars<T, F>(self, fid: NodeId, f: F) -> T where
        F: FnOnce(&[hir::Freevar]) -> T,
    {
        let def_id = self.hir.local_def_id(fid);
        match self.freevars(def_id) {
            None => f(&[]),
            Some(d) => f(&d),
        }
    }
}

fn associated_item<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId)
    -> AssociatedItem
{
    let id = tcx.hir.as_local_node_id(def_id).unwrap();
    let parent_id = tcx.hir.get_parent(id);
    let parent_def_id = tcx.hir.local_def_id(parent_id);
    let parent_item = tcx.hir.expect_item(parent_id);
    match parent_item.node {
        hir::ItemImpl(.., ref impl_item_refs) => {
            if let Some(impl_item_ref) = impl_item_refs.iter().find(|i| i.id.node_id == id) {
                let assoc_item = tcx.associated_item_from_impl_item_ref(parent_def_id,
                                                                        impl_item_ref);
                debug_assert_eq!(assoc_item.def_id, def_id);
                return assoc_item;
            }
        }

        hir::ItemTrait(.., ref trait_item_refs) => {
            if let Some(trait_item_ref) = trait_item_refs.iter().find(|i| i.id.node_id == id) {
                let assoc_item = tcx.associated_item_from_trait_item_ref(parent_def_id,
                                                                         &parent_item.vis,
                                                                         trait_item_ref);
                debug_assert_eq!(assoc_item.def_id, def_id);
                return assoc_item;
            }
        }

        _ => { }
    }

    span_bug!(parent_item.span,
              "unexpected parent of trait or impl item or item not found: {:?}",
              parent_item.node)
}

/// Calculates the Sized-constraint.
///
/// In fact, there are only a few options for the types in the constraint:
///     - an obviously-unsized type
///     - a type parameter or projection whose Sizedness can't be known
///     - a tuple of type parameters or projections, if there are multiple
///       such.
///     - a TyError, if a type contained itself. The representability
///       check should catch this case.
fn adt_sized_constraint<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                  def_id: DefId)
                                  -> &'tcx [Ty<'tcx>] {
    let def = tcx.adt_def(def_id);

    let result = tcx.mk_type_list(def.variants.iter().flat_map(|v| {
        v.fields.last()
    }).flat_map(|f| {
        def.sized_constraint_for_ty(tcx, tcx.type_of(f.did))
    }));

    debug!("adt_sized_constraint: {:?} => {:?}", def, result);

    result
}

fn associated_item_def_ids<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                     def_id: DefId)
                                     -> Lrc<Vec<DefId>> {
    let id = tcx.hir.as_local_node_id(def_id).unwrap();
    let item = tcx.hir.expect_item(id);
    let vec: Vec<_> = match item.node {
        hir::ItemTrait(.., ref trait_item_refs) => {
            trait_item_refs.iter()
                           .map(|trait_item_ref| trait_item_ref.id)
                           .map(|id| tcx.hir.local_def_id(id.node_id))
                           .collect()
        }
        hir::ItemImpl(.., ref impl_item_refs) => {
            impl_item_refs.iter()
                          .map(|impl_item_ref| impl_item_ref.id)
                          .map(|id| tcx.hir.local_def_id(id.node_id))
                          .collect()
        }
        hir::ItemTraitAlias(..) => vec![],
        _ => span_bug!(item.span, "associated_item_def_ids: not impl or trait")
    };
    Lrc::new(vec)
}

fn def_span<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) -> Span {
    tcx.hir.span_if_local(def_id).unwrap()
}

/// If the given def ID describes an item belonging to a trait,
/// return the ID of the trait that the trait item belongs to.
/// Otherwise, return `None`.
fn trait_of_item<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) -> Option<DefId> {
    tcx.opt_associated_item(def_id)
        .and_then(|associated_item| {
            match associated_item.container {
                TraitContainer(def_id) => Some(def_id),
                ImplContainer(_) => None
            }
        })
}

/// See `ParamEnv` struct def'n for details.
fn param_env<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                       def_id: DefId)
                       -> ParamEnv<'tcx> {

    // The param_env of an existential type is its parent's param_env
    if let Some(Def::Existential(_)) = tcx.describe_def(def_id) {
        let parent = tcx.parent_def_id(def_id).expect("impl trait item w/o a parent");
        return param_env(tcx, parent);
    }
    // Compute the bounds on Self and the type parameters.

    let bounds = tcx.predicates_of(def_id).instantiate_identity(tcx);
    let predicates = bounds.predicates;

    // Finally, we have to normalize the bounds in the environment, in
    // case they contain any associated type projections. This process
    // can yield errors if the put in illegal associated types, like
    // `<i32 as Foo>::Bar` where `i32` does not implement `Foo`. We
    // report these errors right here; this doesn't actually feel
    // right to me, because constructing the environment feels like a
    // kind of a "idempotent" action, but I'm not sure where would be
    // a better place. In practice, we construct environments for
    // every fn once during type checking, and we'll abort if there
    // are any errors at that point, so after type checking you can be
    // sure that this will succeed without errors anyway.

    let unnormalized_env = ty::ParamEnv::new(tcx.intern_predicates(&predicates),
                                             traits::Reveal::UserFacing);

    let body_id = tcx.hir.as_local_node_id(def_id).map_or(DUMMY_NODE_ID, |id| {
        tcx.hir.maybe_body_owned_by(id).map_or(id, |body| body.node_id)
    });
    let cause = traits::ObligationCause::misc(tcx.def_span(def_id), body_id);
    traits::normalize_param_env_or_error(tcx, def_id, unnormalized_env, cause)
}

fn crate_disambiguator<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                 crate_num: CrateNum) -> CrateDisambiguator {
    assert_eq!(crate_num, LOCAL_CRATE);
    tcx.sess.local_crate_disambiguator()
}

fn original_crate_name<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                 crate_num: CrateNum) -> Symbol {
    assert_eq!(crate_num, LOCAL_CRATE);
    tcx.crate_name.clone()
}

fn crate_hash<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                        crate_num: CrateNum)
                        -> Svh {
    assert_eq!(crate_num, LOCAL_CRATE);
    tcx.hir.crate_hash
}

fn instance_def_size_estimate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                        instance_def: InstanceDef<'tcx>)
                                        -> usize {
    match instance_def {
        InstanceDef::Item(..) |
        InstanceDef::DropGlue(..) => {
            let mir = tcx.instance_mir(instance_def);
            mir.basic_blocks().iter().map(|bb| bb.statements.len()).sum()
        },
        // Estimate the size of other compiler-generated shims to be 1.
        _ => 1
    }
}

pub fn provide(providers: &mut ty::query::Providers) {
    context::provide(providers);
    erase_regions::provide(providers);
    layout::provide(providers);
    util::provide(providers);
    *providers = ty::query::Providers {
        associated_item,
        associated_item_def_ids,
        adt_sized_constraint,
        def_span,
        param_env,
        trait_of_item,
        crate_disambiguator,
        original_crate_name,
        crate_hash,
        trait_impls_of: trait_def::trait_impls_of_provider,
        instance_def_size_estimate,
        ..*providers
    };
}

/// A map for the local crate mapping each type to a vector of its
/// inherent impls. This is not meant to be used outside of coherence;
/// rather, you should request the vector for a specific type via
/// `tcx.inherent_impls(def_id)` so as to minimize your dependencies
/// (constructing this map requires touching the entire crate).
#[derive(Clone, Debug)]
pub struct CrateInherentImpls {
    pub inherent_impls: DefIdMap<Lrc<Vec<DefId>>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, RustcEncodable, RustcDecodable)]
pub struct SymbolName {
    // FIXME: we don't rely on interning or equality here - better have
    // this be a `&'tcx str`.
    pub name: InternedString
}

impl_stable_hash_for!(struct self::SymbolName {
    name
});

impl SymbolName {
    pub fn new(name: &str) -> SymbolName {
        SymbolName {
            name: Symbol::intern(name).as_interned_str()
        }
    }

    pub fn as_str(&self) -> LocalInternedString {
        self.name.as_str()
    }
}

impl fmt::Display for SymbolName {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.name, fmt)
    }
}

impl fmt::Debug for SymbolName {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.name, fmt)
    }
}
