//! This module contains `TyKind` and its major components.

use hir;
use hir::def_id::DefId;
use infer::canonical::Canonical;
use mir::interpret::ConstValue;
use middle::region;
use polonius_engine::Atom;
use rustc_data_structures::indexed_vec::Idx;
use ty::subst::{Substs, Subst, Kind, UnpackedKind};
use ty::{self, AdtDef, TypeFlags, Ty, TyCtxt, TypeFoldable};
use ty::{List, TyS, ParamEnvAnd, ParamEnv};
use util::captures::Captures;
use mir::interpret::{Scalar, Pointer};

use smallvec::SmallVec;
use std::iter;
use std::cmp::Ordering;
use rustc_target::spec::abi;
use syntax::ast::{self, Ident};
use syntax::symbol::{keywords, InternedString};

use serialize;
use self::InferTy::*;
use self::TyKind::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct TypeAndMut<'tcx> {
    pub ty: Ty<'tcx>,
    pub mutbl: hir::Mutability,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash,
         RustcEncodable, RustcDecodable, Copy)]
/// A "free" region `fr` can be interpreted as "some region
/// at least as big as the scope `fr.scope`".
pub struct FreeRegion {
    pub scope: DefId,
    pub bound_region: BoundRegion,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash,
         RustcEncodable, RustcDecodable, Copy)]
pub enum BoundRegion {
    /// An anonymous region parameter for a given fn (&T)
    BrAnon(u32),

    /// Named region parameters for functions (a in &'a T)
    ///
    /// The def-id is needed to distinguish free regions in
    /// the event of shadowing.
    BrNamed(DefId, InternedString),

    /// Fresh bound identifiers created during GLB computations.
    BrFresh(u32),

    /// Anonymous region for the implicit env pointer parameter
    /// to a closure
    BrEnv,
}

impl BoundRegion {
    pub fn is_named(&self) -> bool {
        match *self {
            BoundRegion::BrNamed(..) => true,
            _ => false,
        }
    }

    /// When canonicalizing, we replace unbound inference variables and free
    /// regions with anonymous late bound regions. This method asserts that
    /// we have an anonymous late bound region, which hence may refer to
    /// a canonical variable.
    pub fn assert_bound_var(&self) -> BoundVar {
        match *self {
            BoundRegion::BrAnon(var) => BoundVar::from_u32(var),
            _ => bug!("bound region is not anonymous"),
        }
    }
}

/// N.B., if you change this, you'll probably want to change the corresponding
/// AST structure in `libsyntax/ast.rs` as well.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub enum TyKind<'tcx> {
    /// The primitive boolean type. Written as `bool`.
    Bool,

    /// The primitive character type; holds a Unicode scalar value
    /// (a non-surrogate code point).  Written as `char`.
    Char,

    /// A primitive signed integer type. For example, `i32`.
    Int(ast::IntTy),

    /// A primitive unsigned integer type. For example, `u32`.
    Uint(ast::UintTy),

    /// A primitive floating-point type. For example, `f64`.
    Float(ast::FloatTy),

    /// Structures, enumerations and unions.
    ///
    /// Substs here, possibly against intuition, *may* contain `Param`s.
    /// That is, even after substitution it is possible that there are type
    /// variables. This happens when the `Adt` corresponds to an ADT
    /// definition and not a concrete use of it.
    Adt(&'tcx AdtDef, &'tcx Substs<'tcx>),

    Foreign(DefId),

    /// The pointee of a string slice. Written as `str`.
    Str,

    /// An array with the given length. Written as `[T; n]`.
    Array(Ty<'tcx>, &'tcx ty::LazyConst<'tcx>),

    /// The pointee of an array slice.  Written as `[T]`.
    Slice(Ty<'tcx>),

    /// A raw pointer. Written as `*mut T` or `*const T`
    RawPtr(TypeAndMut<'tcx>),

    /// A reference; a pointer with an associated lifetime. Written as
    /// `&'a mut T` or `&'a T`.
    Ref(Region<'tcx>, Ty<'tcx>, hir::Mutability),

    /// The anonymous type of a function declaration/definition. Each
    /// function has a unique type, which is output (for a function
    /// named `foo` returning an `i32`) as `fn() -> i32 {foo}`.
    ///
    /// For example the type of `bar` here:
    ///
    /// ```rust
    /// fn foo() -> i32 { 1 }
    /// let bar = foo; // bar: fn() -> i32 {foo}
    /// ```
    FnDef(DefId, &'tcx Substs<'tcx>),

    /// A pointer to a function.  Written as `fn() -> i32`.
    ///
    /// For example the type of `bar` here:
    ///
    /// ```rust
    /// fn foo() -> i32 { 1 }
    /// let bar: fn() -> i32 = foo;
    /// ```
    FnPtr(PolyFnSig<'tcx>),

    /// A trait, defined with `trait`.
    Dynamic(Binder<&'tcx List<ExistentialPredicate<'tcx>>>, ty::Region<'tcx>),

    /// The anonymous type of a closure. Used to represent the type of
    /// `|a| a`.
    Closure(DefId, ClosureSubsts<'tcx>),

    /// The anonymous type of a generator. Used to represent the type of
    /// `|a| yield a`.
    Generator(DefId, GeneratorSubsts<'tcx>, hir::GeneratorMovability),

    /// A type representin the types stored inside a generator.
    /// This should only appear in GeneratorInteriors.
    GeneratorWitness(Binder<&'tcx List<Ty<'tcx>>>),

    /// The never type `!`
    Never,

    /// A tuple type.  For example, `(i32, bool)`.
    Tuple(&'tcx List<Ty<'tcx>>),

    /// The projection of an associated type.  For example,
    /// `<T as Trait<..>>::N`.
    Projection(ProjectionTy<'tcx>),

    /// A placeholder type used when we do not have enough information
    /// to normalize the projection of an associated type to an
    /// existing concrete type. Currently only used with chalk-engine.
    UnnormalizedProjection(ProjectionTy<'tcx>),

    /// Opaque (`impl Trait`) type found in a return type.
    /// The `DefId` comes either from
    /// * the `impl Trait` ast::Ty node,
    /// * or the `existential type` declaration
    /// The substitutions are for the generics of the function in question.
    /// After typeck, the concrete type can be found in the `types` map.
    Opaque(DefId, &'tcx Substs<'tcx>),

    /// A type parameter; for example, `T` in `fn f<T>(x: T) {}
    Param(ParamTy),

    /// Bound type variable, used only when preparing a trait query.
    Bound(ty::DebruijnIndex, BoundTy),

    /// A placeholder type - universally quantified higher-ranked type.
    Placeholder(ty::PlaceholderType),

    /// A type variable used during type checking.
    Infer(InferTy),

    /// A placeholder for a type which could not be computed; this is
    /// propagated to avoid useless error messages.
    Error,
}

// `TyKind` is used a lot. Make sure it doesn't unintentionally get bigger.
#[cfg(target_arch = "x86_64")]
static_assert!(MEM_SIZE_OF_TY_KIND: ::std::mem::size_of::<TyKind<'_>>() == 24);

/// A closure can be modeled as a struct that looks like:
///
///     struct Closure<'l0...'li, T0...Tj, CK, CS, U0...Uk> {
///         upvar0: U0,
///         ...
///         upvark: Uk
///     }
///
/// where:
///
/// - 'l0...'li and T0...Tj are the lifetime and type parameters
///   in scope on the function that defined the closure,
/// - CK represents the *closure kind* (Fn vs FnMut vs FnOnce). This
///   is rather hackily encoded via a scalar type. See
///   `TyS::to_opt_closure_kind` for details.
/// - CS represents the *closure signature*, representing as a `fn()`
///   type. For example, `fn(u32, u32) -> u32` would mean that the closure
///   implements `CK<(u32, u32), Output = u32>`, where `CK` is the trait
///   specified above.
/// - U0...Uk are type parameters representing the types of its upvars
///   (borrowed, if appropriate; that is, if Ui represents a by-ref upvar,
///    and the up-var has the type `Foo`, then `Ui = &Foo`).
///
/// So, for example, given this function:
///
///     fn foo<'a, T>(data: &'a mut T) {
///          do(|| data.count += 1)
///     }
///
/// the type of the closure would be something like:
///
///     struct Closure<'a, T, U0> {
///         data: U0
///     }
///
/// Note that the type of the upvar is not specified in the struct.
/// You may wonder how the impl would then be able to use the upvar,
/// if it doesn't know it's type? The answer is that the impl is
/// (conceptually) not fully generic over Closure but rather tied to
/// instances with the expected upvar types:
///
///     impl<'b, 'a, T> FnMut() for Closure<'a, T, &'b mut &'a mut T> {
///         ...
///     }
///
/// You can see that the *impl* fully specified the type of the upvar
/// and thus knows full well that `data` has type `&'b mut &'a mut T`.
/// (Here, I am assuming that `data` is mut-borrowed.)
///
/// Now, the last question you may ask is: Why include the upvar types
/// as extra type parameters? The reason for this design is that the
/// upvar types can reference lifetimes that are internal to the
/// creating function. In my example above, for example, the lifetime
/// `'b` represents the scope of the closure itself; this is some
/// subset of `foo`, probably just the scope of the call to the to
/// `do()`. If we just had the lifetime/type parameters from the
/// enclosing function, we couldn't name this lifetime `'b`. Note that
/// there can also be lifetimes in the types of the upvars themselves,
/// if one of them happens to be a reference to something that the
/// creating fn owns.
///
/// OK, you say, so why not create a more minimal set of parameters
/// that just includes the extra lifetime parameters? The answer is
/// primarily that it would be hard --- we don't know at the time when
/// we create the closure type what the full types of the upvars are,
/// nor do we know which are borrowed and which are not. In this
/// design, we can just supply a fresh type parameter and figure that
/// out later.
///
/// All right, you say, but why include the type parameters from the
/// original function then? The answer is that codegen may need them
/// when monomorphizing, and they may not appear in the upvars.  A
/// closure could capture no variables but still make use of some
/// in-scope type parameter with a bound (e.g., if our example above
/// had an extra `U: Default`, and the closure called `U::default()`).
///
/// There is another reason. This design (implicitly) prohibits
/// closures from capturing themselves (except via a trait
/// object). This simplifies closure inference considerably, since it
/// means that when we infer the kind of a closure or its upvars, we
/// don't have to handle cycles where the decisions we make for
/// closure C wind up influencing the decisions we ought to make for
/// closure C (which would then require fixed point iteration to
/// handle). Plus it fixes an ICE. :P
///
/// ## Generators
///
/// Perhaps surprisingly, `ClosureSubsts` are also used for
/// generators.  In that case, what is written above is only half-true
/// -- the set of type parameters is similar, but the role of CK and
/// CS are different.  CK represents the "yield type" and CS
/// represents the "return type" of the generator.
///
/// It'd be nice to split this struct into ClosureSubsts and
/// GeneratorSubsts, I believe. -nmatsakis
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct ClosureSubsts<'tcx> {
    /// Lifetime and type parameters from the enclosing function,
    /// concatenated with the types of the upvars.
    ///
    /// These are separated out because codegen wants to pass them around
    /// when monomorphizing.
    pub substs: &'tcx Substs<'tcx>,
}

/// Struct returned by `split()`. Note that these are subslices of the
/// parent slice and not canonical substs themselves.
struct SplitClosureSubsts<'tcx> {
    closure_kind_ty: Ty<'tcx>,
    closure_sig_ty: Ty<'tcx>,
    upvar_kinds: &'tcx [Kind<'tcx>],
}

impl<'tcx> ClosureSubsts<'tcx> {
    /// Divides the closure substs into their respective
    /// components. Single source of truth with respect to the
    /// ordering.
    fn split(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> SplitClosureSubsts<'tcx> {
        let generics = tcx.generics_of(def_id);
        let parent_len = generics.parent_count;
        SplitClosureSubsts {
            closure_kind_ty: self.substs.type_at(parent_len),
            closure_sig_ty: self.substs.type_at(parent_len + 1),
            upvar_kinds: &self.substs[parent_len + 2..],
        }
    }

    #[inline]
    pub fn upvar_tys(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) ->
        impl Iterator<Item=Ty<'tcx>> + 'tcx
    {
        let SplitClosureSubsts { upvar_kinds, .. } = self.split(def_id, tcx);
        upvar_kinds.iter().map(|t| {
            if let UnpackedKind::Type(ty) = t.unpack() {
                ty
            } else {
                bug!("upvar should be type")
            }
        })
    }

    /// Returns the closure kind for this closure; may return a type
    /// variable during inference. To get the closure kind during
    /// inference, use `infcx.closure_kind(def_id, substs)`.
    pub fn closure_kind_ty(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> Ty<'tcx> {
        self.split(def_id, tcx).closure_kind_ty
    }

    /// Returns the type representing the closure signature for this
    /// closure; may contain type variables during inference. To get
    /// the closure signature during inference, use
    /// `infcx.fn_sig(def_id)`.
    pub fn closure_sig_ty(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> Ty<'tcx> {
        self.split(def_id, tcx).closure_sig_ty
    }

    /// Returns the closure kind for this closure; only usable outside
    /// of an inference context, because in that context we know that
    /// there are no type variables.
    ///
    /// If you have an inference context, use `infcx.closure_kind()`.
    pub fn closure_kind(self, def_id: DefId, tcx: TyCtxt<'_, 'tcx, 'tcx>) -> ty::ClosureKind {
        self.split(def_id, tcx).closure_kind_ty.to_opt_closure_kind().unwrap()
    }

    /// Extracts the signature from the closure; only usable outside
    /// of an inference context, because in that context we know that
    /// there are no type variables.
    ///
    /// If you have an inference context, use `infcx.closure_sig()`.
    pub fn closure_sig(self, def_id: DefId, tcx: TyCtxt<'_, 'tcx, 'tcx>) -> ty::PolyFnSig<'tcx> {
        match self.closure_sig_ty(def_id, tcx).sty {
            ty::FnPtr(sig) => sig,
            ref t => bug!("closure_sig_ty is not a fn-ptr: {:?}", t),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct GeneratorSubsts<'tcx> {
    pub substs: &'tcx Substs<'tcx>,
}

struct SplitGeneratorSubsts<'tcx> {
    yield_ty: Ty<'tcx>,
    return_ty: Ty<'tcx>,
    witness: Ty<'tcx>,
    upvar_kinds: &'tcx [Kind<'tcx>],
}

impl<'tcx> GeneratorSubsts<'tcx> {
    fn split(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> SplitGeneratorSubsts<'tcx> {
        let generics = tcx.generics_of(def_id);
        let parent_len = generics.parent_count;
        SplitGeneratorSubsts {
            yield_ty: self.substs.type_at(parent_len),
            return_ty: self.substs.type_at(parent_len + 1),
            witness: self.substs.type_at(parent_len + 2),
            upvar_kinds: &self.substs[parent_len + 3..],
        }
    }

    /// This describes the types that can be contained in a generator.
    /// It will be a type variable initially and unified in the last stages of typeck of a body.
    /// It contains a tuple of all the types that could end up on a generator frame.
    /// The state transformation MIR pass may only produce layouts which mention types
    /// in this tuple. Upvars are not counted here.
    pub fn witness(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> Ty<'tcx> {
        self.split(def_id, tcx).witness
    }

    #[inline]
    pub fn upvar_tys(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) ->
        impl Iterator<Item=Ty<'tcx>> + 'tcx
    {
        let SplitGeneratorSubsts { upvar_kinds, .. } = self.split(def_id, tcx);
        upvar_kinds.iter().map(|t| {
            if let UnpackedKind::Type(ty) = t.unpack() {
                ty
            } else {
                bug!("upvar should be type")
            }
        })
    }

    /// Returns the type representing the yield type of the generator.
    pub fn yield_ty(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> Ty<'tcx> {
        self.split(def_id, tcx).yield_ty
    }

    /// Returns the type representing the return type of the generator.
    pub fn return_ty(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> Ty<'tcx> {
        self.split(def_id, tcx).return_ty
    }

    /// Return the "generator signature", which consists of its yield
    /// and return types.
    ///
    /// NB. Some bits of the code prefers to see this wrapped in a
    /// binder, but it never contains bound regions. Probably this
    /// function should be removed.
    pub fn poly_sig(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> PolyGenSig<'tcx> {
        ty::Binder::dummy(self.sig(def_id, tcx))
    }

    /// Return the "generator signature", which consists of its yield
    /// and return types.
    pub fn sig(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) -> GenSig<'tcx> {
        ty::GenSig {
            yield_ty: self.yield_ty(def_id, tcx),
            return_ty: self.return_ty(def_id, tcx),
        }
    }
}

impl<'a, 'gcx, 'tcx> GeneratorSubsts<'tcx> {
    /// This returns the types of the MIR locals which had to be stored across suspension points.
    /// It is calculated in rustc_mir::transform::generator::StateTransform.
    /// All the types here must be in the tuple in GeneratorInterior.
    pub fn state_tys(
        self,
        def_id: DefId,
        tcx: TyCtxt<'a, 'gcx, 'tcx>,
    ) -> impl Iterator<Item=Ty<'tcx>> + Captures<'gcx> + 'a {
        let state = tcx.generator_layout(def_id).fields.iter();
        state.map(move |d| d.ty.subst(tcx, self.substs))
    }

    /// This is the types of the fields of a generate which
    /// is available before the generator transformation.
    /// It includes the upvars and the state discriminant which is u32.
    pub fn pre_transforms_tys(self, def_id: DefId, tcx: TyCtxt<'a, 'gcx, 'tcx>) ->
        impl Iterator<Item=Ty<'tcx>> + 'a
    {
        self.upvar_tys(def_id, tcx).chain(iter::once(tcx.types.u32))
    }

    /// This is the types of all the fields stored in a generator.
    /// It includes the upvars, state types and the state discriminant which is u32.
    pub fn field_tys(self, def_id: DefId, tcx: TyCtxt<'a, 'gcx, 'tcx>) ->
        impl Iterator<Item=Ty<'tcx>> + Captures<'gcx> + 'a
    {
        self.pre_transforms_tys(def_id, tcx).chain(self.state_tys(def_id, tcx))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum UpvarSubsts<'tcx> {
    Closure(ClosureSubsts<'tcx>),
    Generator(GeneratorSubsts<'tcx>),
}

impl<'tcx> UpvarSubsts<'tcx> {
    #[inline]
    pub fn upvar_tys(self, def_id: DefId, tcx: TyCtxt<'_, '_, '_>) ->
        impl Iterator<Item=Ty<'tcx>> + 'tcx
    {
        let upvar_kinds = match self {
            UpvarSubsts::Closure(substs) => substs.split(def_id, tcx).upvar_kinds,
            UpvarSubsts::Generator(substs) => substs.split(def_id, tcx).upvar_kinds,
        };
        upvar_kinds.iter().map(|t| {
            if let UnpackedKind::Type(ty) = t.unpack() {
                ty
            } else {
                bug!("upvar should be type")
            }
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, RustcEncodable, RustcDecodable)]
pub enum ExistentialPredicate<'tcx> {
    /// e.g., Iterator
    Trait(ExistentialTraitRef<'tcx>),
    /// e.g., Iterator::Item = T
    Projection(ExistentialProjection<'tcx>),
    /// e.g., Send
    AutoTrait(DefId),
}

impl<'a, 'gcx, 'tcx> ExistentialPredicate<'tcx> {
    /// Compares via an ordering that will not change if modules are reordered or other changes are
    /// made to the tree. In particular, this ordering is preserved across incremental compilations.
    pub fn stable_cmp(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, other: &Self) -> Ordering {
        use self::ExistentialPredicate::*;
        match (*self, *other) {
            (Trait(_), Trait(_)) => Ordering::Equal,
            (Projection(ref a), Projection(ref b)) =>
                tcx.def_path_hash(a.item_def_id).cmp(&tcx.def_path_hash(b.item_def_id)),
            (AutoTrait(ref a), AutoTrait(ref b)) =>
                tcx.trait_def(*a).def_path_hash.cmp(&tcx.trait_def(*b).def_path_hash),
            (Trait(_), _) => Ordering::Less,
            (Projection(_), Trait(_)) => Ordering::Greater,
            (Projection(_), _) => Ordering::Less,
            (AutoTrait(_), _) => Ordering::Greater,
        }
    }

}

impl<'a, 'gcx, 'tcx> Binder<ExistentialPredicate<'tcx>> {
    pub fn with_self_ty(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, self_ty: Ty<'tcx>)
        -> ty::Predicate<'tcx> {
        use ty::ToPredicate;
        match *self.skip_binder() {
            ExistentialPredicate::Trait(tr) => Binder(tr).with_self_ty(tcx, self_ty).to_predicate(),
            ExistentialPredicate::Projection(p) =>
                ty::Predicate::Projection(Binder(p.with_self_ty(tcx, self_ty))),
            ExistentialPredicate::AutoTrait(did) => {
                let trait_ref = Binder(ty::TraitRef {
                    def_id: did,
                    substs: tcx.mk_substs_trait(self_ty, &[]),
                });
                trait_ref.to_predicate()
            }
        }
    }
}

impl<'tcx> serialize::UseSpecializedDecodable for &'tcx List<ExistentialPredicate<'tcx>> {}

impl<'tcx> List<ExistentialPredicate<'tcx>> {
    /// Returns the "principal def id" of this set of existential predicates.
    ///
    /// A Rust trait object type consists (in addition to a lifetime bound)
    /// of a set of trait bounds, which are separated into any number
    /// of auto-trait bounds, and at most 1 non-auto-trait bound. The
    /// non-auto-trait bound is called the "principal" of the trait
    /// object.
    ///
    /// Only the principal can have methods or type parameters (because
    /// auto traits can have neither of them). This is important, because
    /// it means the auto traits can be treated as an unordered set (methods
    /// would force an order for the vtable, while relating traits with
    /// type parameters without knowing the order to relate them in is
    /// a rather non-trivial task).
    ///
    /// For example, in the trait object `dyn fmt::Debug + Sync`, the
    /// principal bound is `Some(fmt::Debug)`, while the auto-trait bounds
    /// are the set `{Sync}`.
    ///
    /// It is also possible to have a "trivial" trait object that
    /// consists only of auto traits, with no principal - for example,
    /// `dyn Send + Sync`. In that case, the set of auto-trait bounds
    /// is `{Send, Sync}`, while there is no principal. These trait objects
    /// have a "trivial" vtable consisting of just the size, alignment,
    /// and destructor.
    pub fn principal(&self) -> Option<ExistentialTraitRef<'tcx>> {
        match self[0] {
            ExistentialPredicate::Trait(tr) => Some(tr),
            _ => None
        }
    }

    pub fn principal_def_id(&self) -> Option<DefId> {
        self.principal().map(|d| d.def_id)
    }

    #[inline]
    pub fn projection_bounds<'a>(&'a self) ->
        impl Iterator<Item=ExistentialProjection<'tcx>> + 'a {
        self.iter().filter_map(|predicate| {
            match *predicate {
                ExistentialPredicate::Projection(p) => Some(p),
                _ => None,
            }
        })
    }

    #[inline]
    pub fn auto_traits<'a>(&'a self) -> impl Iterator<Item=DefId> + 'a {
        self.iter().filter_map(|predicate| {
            match *predicate {
                ExistentialPredicate::AutoTrait(d) => Some(d),
                _ => None
            }
        })
    }
}

impl<'tcx> Binder<&'tcx List<ExistentialPredicate<'tcx>>> {
    pub fn principal(&self) -> Option<ty::Binder<ExistentialTraitRef<'tcx>>> {
        self.skip_binder().principal().map(Binder::bind)
    }

    pub fn principal_def_id(&self) -> Option<DefId> {
        self.skip_binder().principal_def_id()
    }

    #[inline]
    pub fn projection_bounds<'a>(&'a self) ->
        impl Iterator<Item=PolyExistentialProjection<'tcx>> + 'a {
        self.skip_binder().projection_bounds().map(Binder::bind)
    }

    #[inline]
    pub fn auto_traits<'a>(&'a self) -> impl Iterator<Item=DefId> + 'a {
        self.skip_binder().auto_traits()
    }

    pub fn iter<'a>(&'a self)
        -> impl DoubleEndedIterator<Item=Binder<ExistentialPredicate<'tcx>>> + 'tcx {
        self.skip_binder().iter().cloned().map(Binder::bind)
    }
}

/// A complete reference to a trait. These take numerous guises in syntax,
/// but perhaps the most recognizable form is in a where clause:
///
///     T: Foo<U>
///
/// This would be represented by a trait-reference where the def-id is the
/// def-id for the trait `Foo` and the substs define `T` as parameter 0,
/// and `U` as parameter 1.
///
/// Trait references also appear in object types like `Foo<U>`, but in
/// that case the `Self` parameter is absent from the substitutions.
///
/// Note that a `TraitRef` introduces a level of region binding, to
/// account for higher-ranked trait bounds like `T: for<'a> Foo<&'a U>`
/// or higher-ranked object types.
#[derive(Copy, Clone, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct TraitRef<'tcx> {
    pub def_id: DefId,
    pub substs: &'tcx Substs<'tcx>,
}

impl<'tcx> TraitRef<'tcx> {
    pub fn new(def_id: DefId, substs: &'tcx Substs<'tcx>) -> TraitRef<'tcx> {
        TraitRef { def_id: def_id, substs: substs }
    }

    /// Returns a `TraitRef` of the form `P0: Foo<P1..Pn>` where `Pi`
    /// are the parameters defined on trait.
    pub fn identity<'a, 'gcx>(tcx: TyCtxt<'a, 'gcx, 'tcx>, def_id: DefId) -> TraitRef<'tcx> {
        TraitRef {
            def_id,
            substs: Substs::identity_for_item(tcx, def_id),
        }
    }

    #[inline]
    pub fn self_ty(&self) -> Ty<'tcx> {
        self.substs.type_at(0)
    }

    pub fn input_types<'a>(&'a self) -> impl DoubleEndedIterator<Item = Ty<'tcx>> + 'a {
        // Select only the "input types" from a trait-reference. For
        // now this is all the types that appear in the
        // trait-reference, but it should eventually exclude
        // associated types.
        self.substs.types()
    }

    pub fn from_method(tcx: TyCtxt<'_, '_, 'tcx>,
                       trait_id: DefId,
                       substs: &Substs<'tcx>)
                       -> ty::TraitRef<'tcx> {
        let defs = tcx.generics_of(trait_id);

        ty::TraitRef {
            def_id: trait_id,
            substs: tcx.intern_substs(&substs[..defs.params.len()])
        }
    }
}

pub type PolyTraitRef<'tcx> = Binder<TraitRef<'tcx>>;

impl<'tcx> PolyTraitRef<'tcx> {
    pub fn self_ty(&self) -> Ty<'tcx> {
        self.skip_binder().self_ty()
    }

    pub fn def_id(&self) -> DefId {
        self.skip_binder().def_id
    }

    pub fn to_poly_trait_predicate(&self) -> ty::PolyTraitPredicate<'tcx> {
        // Note that we preserve binding levels
        Binder(ty::TraitPredicate { trait_ref: self.skip_binder().clone() })
    }
}

/// An existential reference to a trait, where `Self` is erased.
/// For example, the trait object `Trait<'a, 'b, X, Y>` is:
///
///     exists T. T: Trait<'a, 'b, X, Y>
///
/// The substitutions don't include the erased `Self`, only trait
/// type and lifetime parameters (`[X, Y]` and `['a, 'b]` above).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct ExistentialTraitRef<'tcx> {
    pub def_id: DefId,
    pub substs: &'tcx Substs<'tcx>,
}

impl<'a, 'gcx, 'tcx> ExistentialTraitRef<'tcx> {
    pub fn input_types<'b>(&'b self) -> impl DoubleEndedIterator<Item=Ty<'tcx>> + 'b {
        // Select only the "input types" from a trait-reference. For
        // now this is all the types that appear in the
        // trait-reference, but it should eventually exclude
        // associated types.
        self.substs.types()
    }

    pub fn erase_self_ty(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                         trait_ref: ty::TraitRef<'tcx>)
                         -> ty::ExistentialTraitRef<'tcx> {
        // Assert there is a Self.
        trait_ref.substs.type_at(0);

        ty::ExistentialTraitRef {
            def_id: trait_ref.def_id,
            substs: tcx.intern_substs(&trait_ref.substs[1..])
        }
    }

    /// Object types don't have a self-type specified. Therefore, when
    /// we convert the principal trait-ref into a normal trait-ref,
    /// you must give *some* self-type. A common choice is `mk_err()`
    /// or some placeholder type.
    pub fn with_self_ty(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, self_ty: Ty<'tcx>)
        -> ty::TraitRef<'tcx>  {
        // otherwise the escaping vars would be captured by the binder
        // debug_assert!(!self_ty.has_escaping_bound_vars());

        ty::TraitRef {
            def_id: self.def_id,
            substs: tcx.mk_substs_trait(self_ty, self.substs)
        }
    }
}

pub type PolyExistentialTraitRef<'tcx> = Binder<ExistentialTraitRef<'tcx>>;

impl<'tcx> PolyExistentialTraitRef<'tcx> {
    pub fn def_id(&self) -> DefId {
        self.skip_binder().def_id
    }

    /// Object types don't have a self-type specified. Therefore, when
    /// we convert the principal trait-ref into a normal trait-ref,
    /// you must give *some* self-type. A common choice is `mk_err()`
    /// or some placeholder type.
    pub fn with_self_ty(&self, tcx: TyCtxt<'_, '_, 'tcx>,
                        self_ty: Ty<'tcx>)
                        -> ty::PolyTraitRef<'tcx>  {
        self.map_bound(|trait_ref| trait_ref.with_self_ty(tcx, self_ty))
    }
}

/// Binder is a binder for higher-ranked lifetimes or types. It is part of the
/// compiler's representation for things like `for<'a> Fn(&'a isize)`
/// (which would be represented by the type `PolyTraitRef ==
/// Binder<TraitRef>`). Note that when we instantiate,
/// erase, or otherwise "discharge" these bound vars, we change the
/// type from `Binder<T>` to just `T` (see
/// e.g., `liberate_late_bound_regions`).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct Binder<T>(T);

impl<T> Binder<T> {
    /// Wraps `value` in a binder, asserting that `value` does not
    /// contain any bound vars that would be bound by the
    /// binder. This is commonly used to 'inject' a value T into a
    /// different binding level.
    pub fn dummy<'tcx>(value: T) -> Binder<T>
        where T: TypeFoldable<'tcx>
    {
        debug_assert!(!value.has_escaping_bound_vars());
        Binder(value)
    }

    /// Wraps `value` in a binder, binding higher-ranked vars (if any).
    pub fn bind<'tcx>(value: T) -> Binder<T> {
        Binder(value)
    }

    /// Skips the binder and returns the "bound" value. This is a
    /// risky thing to do because it's easy to get confused about
    /// debruijn indices and the like. It is usually better to
    /// discharge the binder using `no_bound_vars` or
    /// `replace_late_bound_regions` or something like
    /// that. `skip_binder` is only valid when you are either
    /// extracting data that has nothing to do with bound vars, you
    /// are doing some sort of test that does not involve bound
    /// regions, or you are being very careful about your depth
    /// accounting.
    ///
    /// Some examples where `skip_binder` is reasonable:
    ///
    /// - extracting the def-id from a PolyTraitRef;
    /// - comparing the self type of a PolyTraitRef to see if it is equal to
    ///   a type parameter `X`, since the type `X` does not reference any regions
    pub fn skip_binder(&self) -> &T {
        &self.0
    }

    pub fn as_ref(&self) -> Binder<&T> {
        Binder(&self.0)
    }

    pub fn map_bound_ref<F, U>(&self, f: F) -> Binder<U>
        where F: FnOnce(&T) -> U
    {
        self.as_ref().map_bound(f)
    }

    pub fn map_bound<F, U>(self, f: F) -> Binder<U>
        where F: FnOnce(T) -> U
    {
        Binder(f(self.0))
    }

    /// Unwraps and returns the value within, but only if it contains
    /// no bound vars at all. (In other words, if this binder --
    /// and indeed any enclosing binder -- doesn't bind anything at
    /// all.) Otherwise, returns `None`.
    ///
    /// (One could imagine having a method that just unwraps a single
    /// binder, but permits late-bound vars bound by enclosing
    /// binders, but that would require adjusting the debruijn
    /// indices, and given the shallow binding structure we often use,
    /// would not be that useful.)
    pub fn no_bound_vars<'tcx>(self) -> Option<T>
        where T: TypeFoldable<'tcx>
    {
        if self.skip_binder().has_escaping_bound_vars() {
            None
        } else {
            Some(self.skip_binder().clone())
        }
    }

    /// Given two things that have the same binder level,
    /// and an operation that wraps on their contents, execute the operation
    /// and then wrap its result.
    ///
    /// `f` should consider bound regions at depth 1 to be free, and
    /// anything it produces with bound regions at depth 1 will be
    /// bound in the resulting return value.
    pub fn fuse<U,F,R>(self, u: Binder<U>, f: F) -> Binder<R>
        where F: FnOnce(T, U) -> R
    {
        Binder(f(self.0, u.0))
    }

    /// Split the contents into two things that share the same binder
    /// level as the original, returning two distinct binders.
    ///
    /// `f` should consider bound regions at depth 1 to be free, and
    /// anything it produces with bound regions at depth 1 will be
    /// bound in the resulting return values.
    pub fn split<U,V,F>(self, f: F) -> (Binder<U>, Binder<V>)
        where F: FnOnce(T) -> (U, V)
    {
        let (u, v) = f(self.0);
        (Binder(u), Binder(v))
    }
}

/// Represents the projection of an associated type. In explicit UFCS
/// form this would be written `<T as Trait<..>>::N`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct ProjectionTy<'tcx> {
    /// The parameters of the associated item.
    pub substs: &'tcx Substs<'tcx>,

    /// The `DefId` of the `TraitItem` for the associated type `N`.
    ///
    /// Note that this is not the `DefId` of the `TraitRef` containing this
    /// associated type, which is in `tcx.associated_item(item_def_id).container`.
    pub item_def_id: DefId,
}

impl<'a, 'tcx> ProjectionTy<'tcx> {
    /// Construct a `ProjectionTy` by searching the trait from `trait_ref` for the
    /// associated item named `item_name`.
    pub fn from_ref_and_name(
        tcx: TyCtxt<'_, '_, '_>, trait_ref: ty::TraitRef<'tcx>, item_name: Ident
    ) -> ProjectionTy<'tcx> {
        let item_def_id = tcx.associated_items(trait_ref.def_id).find(|item| {
            item.kind == ty::AssociatedKind::Type &&
            tcx.hygienic_eq(item_name, item.ident, trait_ref.def_id)
        }).unwrap().def_id;

        ProjectionTy {
            substs: trait_ref.substs,
            item_def_id,
        }
    }

    /// Extracts the underlying trait reference from this projection.
    /// For example, if this is a projection of `<T as Iterator>::Item`,
    /// then this function would return a `T: Iterator` trait reference.
    pub fn trait_ref(&self, tcx: TyCtxt<'_, '_, '_>) -> ty::TraitRef<'tcx> {
        let def_id = tcx.associated_item(self.item_def_id).container.id();
        ty::TraitRef {
            def_id,
            substs: self.substs,
        }
    }

    pub fn self_ty(&self) -> Ty<'tcx> {
        self.substs.type_at(0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable)]
pub struct GenSig<'tcx> {
    pub yield_ty: Ty<'tcx>,
    pub return_ty: Ty<'tcx>,
}

pub type PolyGenSig<'tcx> = Binder<GenSig<'tcx>>;

impl<'tcx> PolyGenSig<'tcx> {
    pub fn yield_ty(&self) -> ty::Binder<Ty<'tcx>> {
        self.map_bound_ref(|sig| sig.yield_ty)
    }
    pub fn return_ty(&self) -> ty::Binder<Ty<'tcx>> {
        self.map_bound_ref(|sig| sig.return_ty)
    }
}

/// Signature of a function type, which I have arbitrarily
/// decided to use to refer to the input/output types.
///
/// - `inputs` is the list of arguments and their modes.
/// - `output` is the return type.
/// - `variadic` indicates whether this is a variadic function. (only true for foreign fns)
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct FnSig<'tcx> {
    pub inputs_and_output: &'tcx List<Ty<'tcx>>,
    pub variadic: bool,
    pub unsafety: hir::Unsafety,
    pub abi: abi::Abi,
}

impl<'tcx> FnSig<'tcx> {
    pub fn inputs(&self) -> &'tcx [Ty<'tcx>] {
        &self.inputs_and_output[..self.inputs_and_output.len() - 1]
    }

    pub fn output(&self) -> Ty<'tcx> {
        self.inputs_and_output[self.inputs_and_output.len() - 1]
    }
}

pub type PolyFnSig<'tcx> = Binder<FnSig<'tcx>>;

impl<'tcx> PolyFnSig<'tcx> {
    #[inline]
    pub fn inputs(&self) -> Binder<&'tcx [Ty<'tcx>]> {
        self.map_bound_ref(|fn_sig| fn_sig.inputs())
    }
    #[inline]
    pub fn input(&self, index: usize) -> ty::Binder<Ty<'tcx>> {
        self.map_bound_ref(|fn_sig| fn_sig.inputs()[index])
    }
    pub fn inputs_and_output(&self) -> ty::Binder<&'tcx List<Ty<'tcx>>> {
        self.map_bound_ref(|fn_sig| fn_sig.inputs_and_output)
    }
    #[inline]
    pub fn output(&self) -> ty::Binder<Ty<'tcx>> {
        self.map_bound_ref(|fn_sig| fn_sig.output())
    }
    pub fn variadic(&self) -> bool {
        self.skip_binder().variadic
    }
    pub fn unsafety(&self) -> hir::Unsafety {
        self.skip_binder().unsafety
    }
    pub fn abi(&self) -> abi::Abi {
        self.skip_binder().abi
    }
}

pub type CanonicalPolyFnSig<'tcx> = Canonical<'tcx, Binder<FnSig<'tcx>>>;


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct ParamTy {
    pub idx: u32,
    pub name: InternedString,
}

impl<'a, 'gcx, 'tcx> ParamTy {
    pub fn new(index: u32, name: InternedString) -> ParamTy {
        ParamTy { idx: index, name: name }
    }

    pub fn for_self() -> ParamTy {
        ParamTy::new(0, keywords::SelfUpper.name().as_interned_str())
    }

    pub fn for_def(def: &ty::GenericParamDef) -> ParamTy {
        ParamTy::new(def.index, def.name)
    }

    pub fn to_ty(self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx> {
        tcx.mk_ty_param(self.idx, self.name)
    }

    pub fn is_self(&self) -> bool {
        // FIXME(#50125): Ignoring `Self` with `idx != 0` might lead to weird behavior elsewhere,
        // but this should only be possible when using `-Z continue-parse-after-error` like
        // `compile-fail/issue-36638.rs`.
        self.name == keywords::SelfUpper.name().as_str() && self.idx == 0
    }
}

/// A [De Bruijn index][dbi] is a standard means of representing
/// regions (and perhaps later types) in a higher-ranked setting. In
/// particular, imagine a type like this:
///
///     for<'a> fn(for<'b> fn(&'b isize, &'a isize), &'a char)
///     ^          ^            |        |         |
///     |          |            |        |         |
///     |          +------------+ 0      |         |
///     |                                |         |
///     +--------------------------------+ 1       |
///     |                                          |
///     +------------------------------------------+ 0
///
/// In this type, there are two binders (the outer fn and the inner
/// fn). We need to be able to determine, for any given region, which
/// fn type it is bound by, the inner or the outer one. There are
/// various ways you can do this, but a De Bruijn index is one of the
/// more convenient and has some nice properties. The basic idea is to
/// count the number of binders, inside out. Some examples should help
/// clarify what I mean.
///
/// Let's start with the reference type `&'b isize` that is the first
/// argument to the inner function. This region `'b` is assigned a De
/// Bruijn index of 0, meaning "the innermost binder" (in this case, a
/// fn). The region `'a` that appears in the second argument type (`&'a
/// isize`) would then be assigned a De Bruijn index of 1, meaning "the
/// second-innermost binder". (These indices are written on the arrays
/// in the diagram).
///
/// What is interesting is that De Bruijn index attached to a particular
/// variable will vary depending on where it appears. For example,
/// the final type `&'a char` also refers to the region `'a` declared on
/// the outermost fn. But this time, this reference is not nested within
/// any other binders (i.e., it is not an argument to the inner fn, but
/// rather the outer one). Therefore, in this case, it is assigned a
/// De Bruijn index of 0, because the innermost binder in that location
/// is the outer fn.
///
/// [dbi]: http://en.wikipedia.org/wiki/De_Bruijn_index
newtype_index! {
    pub struct DebruijnIndex {
        DEBUG_FORMAT = "DebruijnIndex({})",
        const INNERMOST = 0,
    }
}

pub type Region<'tcx> = &'tcx RegionKind;

/// Representation of regions.
///
/// Unlike types, most region variants are "fictitious", not concrete,
/// regions. Among these, `ReStatic`, `ReEmpty` and `ReScope` are the only
/// ones representing concrete regions.
///
/// ## Bound Regions
///
/// These are regions that are stored behind a binder and must be substituted
/// with some concrete region before being used. There are 2 kind of
/// bound regions: early-bound, which are bound in an item's Generics,
/// and are substituted by a Substs,  and late-bound, which are part of
/// higher-ranked types (e.g., `for<'a> fn(&'a ())`) and are substituted by
/// the likes of `liberate_late_bound_regions`. The distinction exists
/// because higher-ranked lifetimes aren't supported in all places. See [1][2].
///
/// Unlike Param-s, bound regions are not supposed to exist "in the wild"
/// outside their binder, e.g., in types passed to type inference, and
/// should first be substituted (by placeholder regions, free regions,
/// or region variables).
///
/// ## Placeholder and Free Regions
///
/// One often wants to work with bound regions without knowing their precise
/// identity. For example, when checking a function, the lifetime of a borrow
/// can end up being assigned to some region parameter. In these cases,
/// it must be ensured that bounds on the region can't be accidentally
/// assumed without being checked.
///
/// To do this, we replace the bound regions with placeholder markers,
/// which don't satisfy any relation not explicitly provided.
///
/// There are 2 kinds of placeholder regions in rustc: `ReFree` and
/// `RePlaceholder`. When checking an item's body, `ReFree` is supposed
/// to be used. These also support explicit bounds: both the internally-stored
/// *scope*, which the region is assumed to outlive, as well as other
/// relations stored in the `FreeRegionMap`. Note that these relations
/// aren't checked when you `make_subregion` (or `eq_types`), only by
/// `resolve_regions_and_report_errors`.
///
/// When working with higher-ranked types, some region relations aren't
/// yet known, so you can't just call `resolve_regions_and_report_errors`.
/// `RePlaceholder` is designed for this purpose. In these contexts,
/// there's also the risk that some inference variable laying around will
/// get unified with your placeholder region: if you want to check whether
/// `for<'a> Foo<'_>: 'a`, and you substitute your bound region `'a`
/// with a placeholder region `'%a`, the variable `'_` would just be
/// instantiated to the placeholder region `'%a`, which is wrong because
/// the inference variable is supposed to satisfy the relation
/// *for every value of the placeholder region*. To ensure that doesn't
/// happen, you can use `leak_check`. This is more clearly explained
/// by the [rustc guide].
///
/// [1]: http://smallcultfollowing.com/babysteps/blog/2013/10/29/intermingled-parameter-lists/
/// [2]: http://smallcultfollowing.com/babysteps/blog/2013/11/04/intermingled-parameter-lists/
/// [rustc guide]: https://rust-lang.github.io/rustc-guide/traits/hrtb.html
#[derive(Clone, PartialEq, Eq, Hash, Copy, RustcEncodable, RustcDecodable, PartialOrd, Ord)]
pub enum RegionKind {
    // Region bound in a type or fn declaration which will be
    // substituted 'early' -- that is, at the same time when type
    // parameters are substituted.
    ReEarlyBound(EarlyBoundRegion),

    // Region bound in a function scope, which will be substituted when the
    // function is called.
    ReLateBound(DebruijnIndex, BoundRegion),

    /// When checking a function body, the types of all arguments and so forth
    /// that refer to bound region parameters are modified to refer to free
    /// region parameters.
    ReFree(FreeRegion),

    /// A concrete region naming some statically determined scope
    /// (e.g., an expression or sequence of statements) within the
    /// current function.
    ReScope(region::Scope),

    /// Static data that has an "infinite" lifetime. Top in the region lattice.
    ReStatic,

    /// A region variable.  Should not exist after typeck.
    ReVar(RegionVid),

    /// A placeholder region - basically the higher-ranked version of ReFree.
    /// Should not exist after typeck.
    RePlaceholder(ty::PlaceholderRegion),

    /// Empty lifetime is for data that is never accessed.
    /// Bottom in the region lattice. We treat ReEmpty somewhat
    /// specially; at least right now, we do not generate instances of
    /// it during the GLB computations, but rather
    /// generate an error instead. This is to improve error messages.
    /// The only way to get an instance of ReEmpty is to have a region
    /// variable with no constraints.
    ReEmpty,

    /// Erased region, used by trait selection, in MIR and during codegen.
    ReErased,

    /// These are regions bound in the "defining type" for a
    /// closure. They are used ONLY as part of the
    /// `ClosureRegionRequirements` that are produced by MIR borrowck.
    /// See `ClosureRegionRequirements` for more details.
    ReClosureBound(RegionVid),
}

impl<'tcx> serialize::UseSpecializedDecodable for Region<'tcx> {}

#[derive(Copy, Clone, PartialEq, Eq, Hash, RustcEncodable, RustcDecodable, Debug, PartialOrd, Ord)]
pub struct EarlyBoundRegion {
    pub def_id: DefId,
    pub index: u32,
    pub name: InternedString,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct TyVid {
    pub index: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct IntVid {
    pub index: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub struct FloatVid {
    pub index: u32,
}

newtype_index! {
    pub struct RegionVid {
        DEBUG_FORMAT = custom,
    }
}

impl Atom for RegionVid {
    fn index(self) -> usize {
        Idx::index(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
pub enum InferTy {
    TyVar(TyVid),
    IntVar(IntVid),
    FloatVar(FloatVid),

    /// A `FreshTy` is one that is generated as a replacement for an
    /// unbound type variable. This is convenient for caching etc. See
    /// `infer::freshen` for more details.
    FreshTy(u32),
    FreshIntTy(u32),
    FreshFloatTy(u32),
}

newtype_index! {
    pub struct BoundVar { .. }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct BoundTy {
    pub var: BoundVar,
    pub kind: BoundTyKind,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub enum BoundTyKind {
    Anon,
    Param(InternedString),
}

impl_stable_hash_for!(struct BoundTy { var, kind });
impl_stable_hash_for!(enum self::BoundTyKind { Anon, Param(a) });

impl From<BoundVar> for BoundTy {
    fn from(var: BoundVar) -> Self {
        BoundTy {
            var,
            kind: BoundTyKind::Anon,
        }
    }
}

/// A `ProjectionPredicate` for an `ExistentialTraitRef`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
pub struct ExistentialProjection<'tcx> {
    pub item_def_id: DefId,
    pub substs: &'tcx Substs<'tcx>,
    pub ty: Ty<'tcx>,
}

pub type PolyExistentialProjection<'tcx> = Binder<ExistentialProjection<'tcx>>;

impl<'a, 'tcx, 'gcx> ExistentialProjection<'tcx> {
    /// Extracts the underlying existential trait reference from this projection.
    /// For example, if this is a projection of `exists T. <T as Iterator>::Item == X`,
    /// then this function would return a `exists T. T: Iterator` existential trait
    /// reference.
    pub fn trait_ref(&self, tcx: TyCtxt<'_, '_, '_>) -> ty::ExistentialTraitRef<'tcx> {
        let def_id = tcx.associated_item(self.item_def_id).container.id();
        ty::ExistentialTraitRef{
            def_id,
            substs: self.substs,
        }
    }

    pub fn with_self_ty(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                        self_ty: Ty<'tcx>)
                        -> ty::ProjectionPredicate<'tcx>
    {
        // otherwise the escaping regions would be captured by the binders
        debug_assert!(!self_ty.has_escaping_bound_vars());

        ty::ProjectionPredicate {
            projection_ty: ty::ProjectionTy {
                item_def_id: self.item_def_id,
                substs: tcx.mk_substs_trait(self_ty, self.substs),
            },
            ty: self.ty,
        }
    }
}

impl<'a, 'tcx, 'gcx> PolyExistentialProjection<'tcx> {
    pub fn with_self_ty(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, self_ty: Ty<'tcx>)
        -> ty::PolyProjectionPredicate<'tcx> {
        self.map_bound(|p| p.with_self_ty(tcx, self_ty))
    }

    pub fn item_def_id(&self) -> DefId {
        return self.skip_binder().item_def_id;
    }
}

impl DebruijnIndex {
    /// Returns the resulting index when this value is moved into
    /// `amount` number of new binders. So e.g., if you had
    ///
    ///    for<'a> fn(&'a x)
    ///
    /// and you wanted to change to
    ///
    ///    for<'a> fn(for<'b> fn(&'a x))
    ///
    /// you would need to shift the index for `'a` into a new binder.
    #[must_use]
    pub fn shifted_in(self, amount: u32) -> DebruijnIndex {
        DebruijnIndex::from_u32(self.as_u32() + amount)
    }

    /// Update this index in place by shifting it "in" through
    /// `amount` number of binders.
    pub fn shift_in(&mut self, amount: u32) {
        *self = self.shifted_in(amount);
    }

    /// Returns the resulting index when this value is moved out from
    /// `amount` number of new binders.
    #[must_use]
    pub fn shifted_out(self, amount: u32) -> DebruijnIndex {
        DebruijnIndex::from_u32(self.as_u32() - amount)
    }

    /// Update in place by shifting out from `amount` binders.
    pub fn shift_out(&mut self, amount: u32) {
        *self = self.shifted_out(amount);
    }

    /// Adjusts any Debruijn Indices so as to make `to_binder` the
    /// innermost binder. That is, if we have something bound at `to_binder`,
    /// it will now be bound at INNERMOST. This is an appropriate thing to do
    /// when moving a region out from inside binders:
    ///
    /// ```
    ///             for<'a>   fn(for<'b>   for<'c>   fn(&'a u32), _)
    /// // Binder:  D3           D2        D1            ^^
    /// ```
    ///
    /// Here, the region `'a` would have the debruijn index D3,
    /// because it is the bound 3 binders out. However, if we wanted
    /// to refer to that region `'a` in the second argument (the `_`),
    /// those two binders would not be in scope. In that case, we
    /// might invoke `shift_out_to_binder(D3)`. This would adjust the
    /// debruijn index of `'a` to D1 (the innermost binder).
    ///
    /// If we invoke `shift_out_to_binder` and the region is in fact
    /// bound by one of the binders we are shifting out of, that is an
    /// error (and should fail an assertion failure).
    pub fn shifted_out_to_binder(self, to_binder: DebruijnIndex) -> Self {
        self.shifted_out(to_binder.as_u32() - INNERMOST.as_u32())
    }
}

impl_stable_hash_for!(struct DebruijnIndex { private });

/// Region utilities
impl RegionKind {
    /// Is this region named by the user?
    pub fn has_name(&self) -> bool {
        match *self {
            RegionKind::ReEarlyBound(ebr) => ebr.has_name(),
            RegionKind::ReLateBound(_, br) => br.is_named(),
            RegionKind::ReFree(fr) => fr.bound_region.is_named(),
            RegionKind::ReScope(..) => false,
            RegionKind::ReStatic => true,
            RegionKind::ReVar(..) => false,
            RegionKind::RePlaceholder(placeholder) => placeholder.name.is_named(),
            RegionKind::ReEmpty => false,
            RegionKind::ReErased => false,
            RegionKind::ReClosureBound(..) => false,
        }
    }

    pub fn is_late_bound(&self) -> bool {
        match *self {
            ty::ReLateBound(..) => true,
            _ => false,
        }
    }

    pub fn is_placeholder(&self) -> bool {
        match *self {
            ty::RePlaceholder(..) => true,
            _ => false,
        }
    }

    pub fn bound_at_or_above_binder(&self, index: DebruijnIndex) -> bool {
        match *self {
            ty::ReLateBound(debruijn, _) => debruijn >= index,
            _ => false,
        }
    }

    /// Adjusts any Debruijn Indices so as to make `to_binder` the
    /// innermost binder. That is, if we have something bound at `to_binder`,
    /// it will now be bound at INNERMOST. This is an appropriate thing to do
    /// when moving a region out from inside binders:
    ///
    /// ```
    ///             for<'a>   fn(for<'b>   for<'c>   fn(&'a u32), _)
    /// // Binder:  D3           D2        D1            ^^
    /// ```
    ///
    /// Here, the region `'a` would have the debruijn index D3,
    /// because it is the bound 3 binders out. However, if we wanted
    /// to refer to that region `'a` in the second argument (the `_`),
    /// those two binders would not be in scope. In that case, we
    /// might invoke `shift_out_to_binder(D3)`. This would adjust the
    /// debruijn index of `'a` to D1 (the innermost binder).
    ///
    /// If we invoke `shift_out_to_binder` and the region is in fact
    /// bound by one of the binders we are shifting out of, that is an
    /// error (and should fail an assertion failure).
    pub fn shifted_out_to_binder(&self, to_binder: ty::DebruijnIndex) -> RegionKind {
        match *self {
            ty::ReLateBound(debruijn, r) => ty::ReLateBound(
                debruijn.shifted_out_to_binder(to_binder),
                r,
            ),
            r => r
        }
    }

    pub fn keep_in_local_tcx(&self) -> bool {
        if let ty::ReVar(..) = self {
            true
        } else {
            false
        }
    }

    pub fn type_flags(&self) -> TypeFlags {
        let mut flags = TypeFlags::empty();

        if self.keep_in_local_tcx() {
            flags = flags | TypeFlags::KEEP_IN_LOCAL_TCX;
        }

        match *self {
            ty::ReVar(..) => {
                flags = flags | TypeFlags::HAS_FREE_REGIONS;
                flags = flags | TypeFlags::HAS_RE_INFER;
            }
            ty::RePlaceholder(..) => {
                flags = flags | TypeFlags::HAS_FREE_REGIONS;
                flags = flags | TypeFlags::HAS_RE_PLACEHOLDER;
            }
            ty::ReLateBound(..) => {
                flags = flags | TypeFlags::HAS_RE_LATE_BOUND;
            }
            ty::ReEarlyBound(..) => {
                flags = flags | TypeFlags::HAS_FREE_REGIONS;
                flags = flags | TypeFlags::HAS_RE_EARLY_BOUND;
            }
            ty::ReEmpty |
            ty::ReStatic |
            ty::ReFree { .. } |
            ty::ReScope { .. } => {
                flags = flags | TypeFlags::HAS_FREE_REGIONS;
            }
            ty::ReErased => {
            }
            ty::ReClosureBound(..) => {
                flags = flags | TypeFlags::HAS_FREE_REGIONS;
            }
        }

        match *self {
            ty::ReStatic | ty::ReEmpty | ty::ReErased | ty::ReLateBound(..) => (),
            _ => flags = flags | TypeFlags::HAS_FREE_LOCAL_NAMES,
        }

        debug!("type_flags({:?}) = {:?}", self, flags);

        flags
    }

    /// Given an early-bound or free region, returns the def-id where it was bound.
    /// For example, consider the regions in this snippet of code:
    ///
    /// ```
    /// impl<'a> Foo {
    ///      ^^ -- early bound, declared on an impl
    ///
    ///     fn bar<'b, 'c>(x: &self, y: &'b u32, z: &'c u64) where 'static: 'c
    ///            ^^  ^^     ^ anonymous, late-bound
    ///            |   early-bound, appears in where-clauses
    ///            late-bound, appears only in fn args
    ///     {..}
    /// }
    /// ```
    ///
    /// Here, `free_region_binding_scope('a)` would return the def-id
    /// of the impl, and for all the other highlighted regions, it
    /// would return the def-id of the function. In other cases (not shown), this
    /// function might return the def-id of a closure.
    pub fn free_region_binding_scope(&self, tcx: TyCtxt<'_, '_, '_>) -> DefId {
        match self {
            ty::ReEarlyBound(br) => {
                tcx.parent_def_id(br.def_id).unwrap()
            }
            ty::ReFree(fr) => fr.scope,
            _ => bug!("free_region_binding_scope invoked on inappropriate region: {:?}", self),
        }
    }
}

/// Type utilities
impl<'a, 'gcx, 'tcx> TyS<'tcx> {
    pub fn is_unit(&self) -> bool {
        match self.sty {
            Tuple(ref tys) => tys.is_empty(),
            _ => false,
        }
    }

    pub fn is_never(&self) -> bool {
        match self.sty {
            Never => true,
            _ => false,
        }
    }

    /// Checks whether a type is definitely uninhabited. This is
    /// conservative: for some types that are uninhabited we return `false`,
    /// but we only return `true` for types that are definitely uninhabited.
    /// `ty.conservative_is_privately_uninhabited` implies that any value of type `ty`
    /// will be `Abi::Uninhabited`. (Note that uninhabited types may have nonzero
    /// size, to account for partial initialisation. See #49298 for details.)
    pub fn conservative_is_privately_uninhabited(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> bool {
        // FIXME(varkor): we can make this less conversative by substituting concrete
        // type arguments.
        match self.sty {
            ty::Never => true,
            ty::Adt(def, _) if def.is_union() => {
                // For now, `union`s are never considered uninhabited.
                false
            }
            ty::Adt(def, _) => {
                // Any ADT is uninhabited if either:
                // (a) It has no variants (i.e. an empty `enum`);
                // (b) Each of its variants (a single one in the case of a `struct`) has at least
                //     one uninhabited field.
                def.variants.iter().all(|var| {
                    var.fields.iter().any(|field| {
                        tcx.type_of(field.did).conservative_is_privately_uninhabited(tcx)
                    })
                })
            }
            ty::Tuple(tys) => tys.iter().any(|ty| ty.conservative_is_privately_uninhabited(tcx)),
            ty::Array(ty, len) => {
                match len.assert_usize(tcx) {
                    // If the array is definitely non-empty, it's uninhabited if
                    // the type of its elements is uninhabited.
                    Some(n) if n != 0 => ty.conservative_is_privately_uninhabited(tcx),
                    _ => false
                }
            }
            ty::Ref(..) => {
                // References to uninitialised memory is valid for any type, including
                // uninhabited types, in unsafe code, so we treat all references as
                // inhabited.
                false
            }
            _ => false,
        }
    }

    pub fn is_primitive(&self) -> bool {
        match self.sty {
            Bool | Char | Int(_) | Uint(_) | Float(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_ty_var(&self) -> bool {
        match self.sty {
            Infer(TyVar(_)) => true,
            _ => false,
        }
    }

    pub fn is_ty_infer(&self) -> bool {
        match self.sty {
            Infer(_) => true,
            _ => false,
        }
    }

    pub fn is_phantom_data(&self) -> bool {
        if let Adt(def, _) = self.sty {
            def.is_phantom_data()
        } else {
            false
        }
    }

    pub fn is_bool(&self) -> bool { self.sty == Bool }

    pub fn is_param(&self, index: u32) -> bool {
        match self.sty {
            ty::Param(ref data) => data.idx == index,
            _ => false,
        }
    }

    pub fn is_self(&self) -> bool {
        match self.sty {
            Param(ref p) => p.is_self(),
            _ => false,
        }
    }

    pub fn is_slice(&self) -> bool {
        match self.sty {
            RawPtr(TypeAndMut { ty, .. }) | Ref(_, ty, _) => match ty.sty {
                Slice(_) | Str => true,
                _ => false,
            },
            _ => false
        }
    }

    #[inline]
    pub fn is_simd(&self) -> bool {
        match self.sty {
            Adt(def, _) => def.repr.simd(),
            _ => false,
        }
    }

    pub fn sequence_element_type(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx> {
        match self.sty {
            Array(ty, _) | Slice(ty) => ty,
            Str => tcx.mk_mach_uint(ast::UintTy::U8),
            _ => bug!("sequence_element_type called on non-sequence value: {}", self),
        }
    }

    pub fn simd_type(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Ty<'tcx> {
        match self.sty {
            Adt(def, substs) => {
                def.non_enum_variant().fields[0].ty(tcx, substs)
            }
            _ => bug!("simd_type called on invalid type")
        }
    }

    pub fn simd_size(&self, _cx: TyCtxt<'_, '_, '_>) -> usize {
        match self.sty {
            Adt(def, _) => def.non_enum_variant().fields.len(),
            _ => bug!("simd_size called on invalid type")
        }
    }

    pub fn is_region_ptr(&self) -> bool {
        match self.sty {
            Ref(..) => true,
            _ => false,
        }
    }

    pub fn is_mutable_pointer(&self) -> bool {
        match self.sty {
            RawPtr(TypeAndMut { mutbl: hir::Mutability::MutMutable, .. }) |
            Ref(_, _, hir::Mutability::MutMutable) => true,
            _ => false
        }
    }

    pub fn is_unsafe_ptr(&self) -> bool {
        match self.sty {
            RawPtr(_) => return true,
            _ => return false,
        }
    }

    /// Returns `true` if this type is an `Arc<T>`.
    pub fn is_arc(&self) -> bool {
        match self.sty {
            Adt(def, _) => def.is_arc(),
            _ => false,
        }
    }

    /// Returns `true` if this type is an `Rc<T>`.
    pub fn is_rc(&self) -> bool {
        match self.sty {
            Adt(def, _) => def.is_rc(),
            _ => false,
        }
    }

    pub fn is_box(&self) -> bool {
        match self.sty {
            Adt(def, _) => def.is_box(),
            _ => false,
        }
    }

    /// panics if called on any type other than `Box<T>`
    pub fn boxed_ty(&self) -> Ty<'tcx> {
        match self.sty {
            Adt(def, substs) if def.is_box() => substs.type_at(0),
            _ => bug!("`boxed_ty` is called on non-box type {:?}", self),
        }
    }

    /// A scalar type is one that denotes an atomic datum, with no sub-components.
    /// (A RawPtr is scalar because it represents a non-managed pointer, so its
    /// contents are abstract to rustc.)
    pub fn is_scalar(&self) -> bool {
        match self.sty {
            Bool | Char | Int(_) | Float(_) | Uint(_) |
            Infer(IntVar(_)) | Infer(FloatVar(_)) |
            FnDef(..) | FnPtr(_) | RawPtr(_) => true,
            _ => false
        }
    }

    /// Returns true if this type is a floating point type and false otherwise.
    pub fn is_floating_point(&self) -> bool {
        match self.sty {
            Float(_) |
            Infer(FloatVar(_)) => true,
            _ => false,
        }
    }

    pub fn is_trait(&self) -> bool {
        match self.sty {
            Dynamic(..) => true,
            _ => false,
        }
    }

    pub fn is_enum(&self) -> bool {
        match self.sty {
            Adt(adt_def, _) => {
                adt_def.is_enum()
            }
            _ => false,
        }
    }

    pub fn is_closure(&self) -> bool {
        match self.sty {
            Closure(..) => true,
            _ => false,
        }
    }

    pub fn is_generator(&self) -> bool {
        match self.sty {
            Generator(..) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_integral(&self) -> bool {
        match self.sty {
            Infer(IntVar(_)) | Int(_) | Uint(_) => true,
            _ => false
        }
    }

    pub fn is_fresh_ty(&self) -> bool {
        match self.sty {
            Infer(FreshTy(_)) => true,
            _ => false,
        }
    }

    pub fn is_fresh(&self) -> bool {
        match self.sty {
            Infer(FreshTy(_)) => true,
            Infer(FreshIntTy(_)) => true,
            Infer(FreshFloatTy(_)) => true,
            _ => false,
        }
    }

    pub fn is_char(&self) -> bool {
        match self.sty {
            Char => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_fp(&self) -> bool {
        match self.sty {
            Infer(FloatVar(_)) | Float(_) => true,
            _ => false
        }
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integral() || self.is_fp()
    }

    pub fn is_signed(&self) -> bool {
        match self.sty {
            Int(_) => true,
            _ => false,
        }
    }

    pub fn is_pointer_sized(&self) -> bool {
        match self.sty {
            Int(ast::IntTy::Isize) | Uint(ast::UintTy::Usize) => true,
            _ => false,
        }
    }

    pub fn is_machine(&self) -> bool {
        match self.sty {
            Int(ast::IntTy::Isize) | Uint(ast::UintTy::Usize) => false,
            Int(..) | Uint(..) | Float(..) => true,
            _ => false,
        }
    }

    pub fn has_concrete_skeleton(&self) -> bool {
        match self.sty {
            Param(_) | Infer(_) | Error => false,
            _ => true,
        }
    }

    /// Returns the type and mutability of `*ty`.
    ///
    /// The parameter `explicit` indicates if this is an *explicit* dereference.
    /// Some types -- notably unsafe ptrs -- can only be dereferenced explicitly.
    pub fn builtin_deref(&self, explicit: bool) -> Option<TypeAndMut<'tcx>> {
        match self.sty {
            Adt(def, _) if def.is_box() => {
                Some(TypeAndMut {
                    ty: self.boxed_ty(),
                    mutbl: hir::MutImmutable,
                })
            },
            Ref(_, ty, mutbl) => Some(TypeAndMut { ty, mutbl }),
            RawPtr(mt) if explicit => Some(mt),
            _ => None,
        }
    }

    /// Returns the type of `ty[i]`.
    pub fn builtin_index(&self) -> Option<Ty<'tcx>> {
        match self.sty {
            Array(ty, _) | Slice(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn fn_sig(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>) -> PolyFnSig<'tcx> {
        match self.sty {
            FnDef(def_id, substs) => {
                tcx.fn_sig(def_id).subst(tcx, substs)
            }
            FnPtr(f) => f,
            _ => bug!("Ty::fn_sig() called on non-fn type: {:?}", self)
        }
    }

    pub fn is_fn(&self) -> bool {
        match self.sty {
            FnDef(..) | FnPtr(_) => true,
            _ => false,
        }
    }

    pub fn is_impl_trait(&self) -> bool {
        match self.sty {
            Opaque(..) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn ty_adt_def(&self) -> Option<&'tcx AdtDef> {
        match self.sty {
            Adt(adt, _) => Some(adt),
            _ => None,
        }
    }

    /// Push onto `out` the regions directly referenced from this type (but not
    /// types reachable from this type via `walk_tys`). This ignores late-bound
    /// regions binders.
    pub fn push_regions(&self, out: &mut SmallVec<[ty::Region<'tcx>; 4]>) {
        match self.sty {
            Ref(region, _, _) => {
                out.push(region);
            }
            Dynamic(ref obj, region) => {
                out.push(region);
                if let Some(principal) = obj.principal() {
                    out.extend(principal.skip_binder().substs.regions());
                }
            }
            Adt(_, substs) | Opaque(_, substs) => {
                out.extend(substs.regions())
            }
            Closure(_, ClosureSubsts { ref substs }) |
            Generator(_, GeneratorSubsts { ref substs }, _) => {
                out.extend(substs.regions())
            }
            Projection(ref data) | UnnormalizedProjection(ref data) => {
                out.extend(data.substs.regions())
            }
            FnDef(..) |
            FnPtr(_) |
            GeneratorWitness(..) |
            Bool |
            Char |
            Int(_) |
            Uint(_) |
            Float(_) |
            Str |
            Array(..) |
            Slice(_) |
            RawPtr(_) |
            Never |
            Tuple(..) |
            Foreign(..) |
            Param(_) |
            Bound(..) |
            Placeholder(..) |
            Infer(_) |
            Error => {}
        }
    }

    /// When we create a closure, we record its kind (i.e., what trait
    /// it implements) into its `ClosureSubsts` using a type
    /// parameter. This is kind of a phantom type, except that the
    /// most convenient thing for us to are the integral types. This
    /// function converts such a special type into the closure
    /// kind. To go the other way, use
    /// `tcx.closure_kind_ty(closure_kind)`.
    ///
    /// Note that during type checking, we use an inference variable
    /// to represent the closure kind, because it has not yet been
    /// inferred. Once upvar inference (in `src/librustc_typeck/check/upvar.rs`)
    /// is complete, that type variable will be unified.
    pub fn to_opt_closure_kind(&self) -> Option<ty::ClosureKind> {
        match self.sty {
            Int(int_ty) => match int_ty {
                ast::IntTy::I8 => Some(ty::ClosureKind::Fn),
                ast::IntTy::I16 => Some(ty::ClosureKind::FnMut),
                ast::IntTy::I32 => Some(ty::ClosureKind::FnOnce),
                _ => bug!("cannot convert type `{:?}` to a closure kind", self),
            },

            Infer(_) => None,

            Error => Some(ty::ClosureKind::Fn),

            _ => bug!("cannot convert type `{:?}` to a closure kind", self),
        }
    }

    /// Fast path helper for testing if a type is `Sized`.
    ///
    /// Returning true means the type is known to be sized. Returning
    /// `false` means nothing -- could be sized, might not be.
    pub fn is_trivially_sized(&self, tcx: TyCtxt<'_, '_, 'tcx>) -> bool {
        match self.sty {
            ty::Infer(ty::IntVar(_)) | ty::Infer(ty::FloatVar(_)) |
            ty::Uint(_) | ty::Int(_) | ty::Bool | ty::Float(_) |
            ty::FnDef(..) | ty::FnPtr(_) | ty::RawPtr(..) |
            ty::Char | ty::Ref(..) | ty::Generator(..) |
            ty::GeneratorWitness(..) | ty::Array(..) | ty::Closure(..) |
            ty::Never | ty::Error =>
                true,

            ty::Str | ty::Slice(_) | ty::Dynamic(..) | ty::Foreign(..) =>
                false,

            ty::Tuple(tys) =>
                tys.iter().all(|ty| ty.is_trivially_sized(tcx)),

            ty::Adt(def, _substs) =>
                def.sized_constraint(tcx).is_empty(),

            ty::Projection(_) | ty::Param(_) | ty::Opaque(..) => false,

            ty::UnnormalizedProjection(..) => bug!("only used with chalk-engine"),

            ty::Infer(ty::TyVar(_)) => false,

            ty::Bound(..) |
            ty::Placeholder(..) |
            ty::Infer(ty::FreshTy(_)) |
            ty::Infer(ty::FreshIntTy(_)) |
            ty::Infer(ty::FreshFloatTy(_)) =>
                bug!("is_trivially_sized applied to unexpected type: {:?}", self),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, RustcEncodable, RustcDecodable, Eq, PartialEq, Ord, PartialOrd)]
/// Used in the HIR by using `Unevaluated` everywhere and later normalizing to `Evaluated` if the
/// code is monomorphic enough for that.
pub enum LazyConst<'tcx> {
    Unevaluated(DefId, &'tcx Substs<'tcx>),
    Evaluated(Const<'tcx>),
}

impl<'tcx> LazyConst<'tcx> {
    pub fn map_evaluated<R>(self, f: impl FnOnce(Const<'tcx>) -> Option<R>) -> Option<R> {
        match self {
            LazyConst::Evaluated(c) => f(c),
            LazyConst::Unevaluated(..) => None,
        }
    }

    pub fn assert_usize(self, tcx: TyCtxt<'_, '_, '_>) -> Option<u64> {
        self.map_evaluated(|c| c.assert_usize(tcx))
    }

    #[inline]
    pub fn unwrap_usize(&self, tcx: TyCtxt<'_, '_, '_>) -> u64 {
        self.assert_usize(tcx).expect("expected `LazyConst` to contain a usize")
    }
}

/// Typed constant value.
#[derive(Copy, Clone, Debug, Hash, RustcEncodable, RustcDecodable, Eq, PartialEq, Ord, PartialOrd)]
pub struct Const<'tcx> {
    pub ty: Ty<'tcx>,

    pub val: ConstValue<'tcx>,
}

impl<'tcx> Const<'tcx> {
    #[inline]
    pub fn from_scalar(
        val: Scalar,
        ty: Ty<'tcx>,
    ) -> Self {
        Self {
            val: ConstValue::Scalar(val),
            ty,
        }
    }

    #[inline]
    pub fn from_bits(
        tcx: TyCtxt<'_, '_, 'tcx>,
        bits: u128,
        ty: ParamEnvAnd<'tcx, Ty<'tcx>>,
    ) -> Self {
        let ty = tcx.lift_to_global(&ty).unwrap();
        let size = tcx.layout_of(ty).unwrap_or_else(|e| {
            panic!("could not compute layout for {:?}: {:?}", ty, e)
        }).size;
        let shift = 128 - size.bits();
        let truncated = (bits << shift) >> shift;
        assert_eq!(truncated, bits, "from_bits called with untruncated value");
        Self::from_scalar(Scalar::Bits { bits, size: size.bytes() as u8 }, ty.value)
    }

    #[inline]
    pub fn zero_sized(ty: Ty<'tcx>) -> Self {
        Self::from_scalar(Scalar::Bits { bits: 0, size: 0 }, ty)
    }

    #[inline]
    pub fn from_bool(tcx: TyCtxt<'_, '_, 'tcx>, v: bool) -> Self {
        Self::from_bits(tcx, v as u128, ParamEnv::empty().and(tcx.types.bool))
    }

    #[inline]
    pub fn from_usize(tcx: TyCtxt<'_, '_, 'tcx>, n: u64) -> Self {
        Self::from_bits(tcx, n as u128, ParamEnv::empty().and(tcx.types.usize))
    }

    #[inline]
    pub fn to_bits(
        &self,
        tcx: TyCtxt<'_, '_, 'tcx>,
        ty: ParamEnvAnd<'tcx, Ty<'tcx>>,
    ) -> Option<u128> {
        if self.ty != ty.value {
            return None;
        }
        let ty = tcx.lift_to_global(&ty).unwrap();
        let size = tcx.layout_of(ty).ok()?.size;
        self.val.try_to_bits(size)
    }

    #[inline]
    pub fn to_ptr(&self) -> Option<Pointer> {
        self.val.try_to_ptr()
    }

    #[inline]
    pub fn assert_bits(
        &self,
        tcx: TyCtxt<'_, '_, '_>,
        ty: ParamEnvAnd<'tcx, Ty<'tcx>>,
    ) -> Option<u128> {
        assert_eq!(self.ty, ty.value);
        let ty = tcx.lift_to_global(&ty).unwrap();
        let size = tcx.layout_of(ty).ok()?.size;
        self.val.try_to_bits(size)
    }

    #[inline]
    pub fn assert_bool(&self, tcx: TyCtxt<'_, '_, '_>) -> Option<bool> {
        self.assert_bits(tcx, ParamEnv::empty().and(tcx.types.bool)).and_then(|v| match v {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        })
    }

    #[inline]
    pub fn assert_usize(&self, tcx: TyCtxt<'_, '_, '_>) -> Option<u64> {
        self.assert_bits(tcx, ParamEnv::empty().and(tcx.types.usize)).map(|v| v as u64)
    }

    #[inline]
    pub fn unwrap_bits(
        &self,
        tcx: TyCtxt<'_, '_, '_>,
        ty: ParamEnvAnd<'tcx, Ty<'tcx>>,
    ) -> u128 {
        self.assert_bits(tcx, ty).unwrap_or_else(||
            bug!("expected bits of {}, got {:#?}", ty.value, self))
    }

    #[inline]
    pub fn unwrap_usize(&self, tcx: TyCtxt<'_, '_, '_>) -> u64 {
        self.assert_usize(tcx).unwrap_or_else(||
            bug!("expected constant usize, got {:#?}", self))
    }
}

impl<'tcx> serialize::UseSpecializedDecodable for &'tcx LazyConst<'tcx> {}
