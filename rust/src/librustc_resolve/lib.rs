#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://doc.rust-lang.org/favicon.ico",
       html_root_url = "https://doc.rust-lang.org/nightly/")]

#![feature(crate_visibility_modifier)]
#![feature(label_break_value)]
#![feature(nll)]
#![feature(rustc_diagnostic_macros)]
#![feature(slice_sort_by_cached_key)]

#![recursion_limit="256"]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;
#[macro_use]
extern crate syntax;
extern crate syntax_pos;
extern crate rustc_errors as errors;
extern crate arena;
#[macro_use]
extern crate rustc;
extern crate rustc_data_structures;
extern crate rustc_metadata;

pub use rustc::hir::def::{Namespace, PerNS};

use self::TypeParameters::*;
use self::RibKind::*;

use rustc::hir::map::{Definitions, DefCollector};
use rustc::hir::{self, PrimTy, Bool, Char, Float, Int, Uint, Str};
use rustc::middle::cstore::CrateStore;
use rustc::session::Session;
use rustc::lint;
use rustc::hir::def::*;
use rustc::hir::def::Namespace::*;
use rustc::hir::def_id::{CRATE_DEF_INDEX, LOCAL_CRATE, DefId};
use rustc::hir::{Freevar, FreevarMap, TraitCandidate, TraitMap, GlobMap};
use rustc::session::config::nightly_options;
use rustc::ty;
use rustc::util::nodemap::{NodeMap, NodeSet, FxHashMap, FxHashSet, DefIdMap};

use rustc_metadata::creader::CrateLoader;
use rustc_metadata::cstore::CStore;

use syntax::source_map::SourceMap;
use syntax::ext::hygiene::{Mark, Transparency, SyntaxContext};
use syntax::ast::{self, Name, NodeId, Ident, FloatTy, IntTy, UintTy};
use syntax::ext::base::SyntaxExtension;
use syntax::ext::base::Determinacy::{self, Determined, Undetermined};
use syntax::ext::base::MacroKind;
use syntax::symbol::{Symbol, keywords};
use syntax::util::lev_distance::find_best_match_for_name;

use syntax::visit::{self, FnKind, Visitor};
use syntax::attr;
use syntax::ast::{CRATE_NODE_ID, Arm, IsAsync, BindingMode, Block, Crate, Expr, ExprKind};
use syntax::ast::{FnDecl, ForeignItem, ForeignItemKind, GenericParamKind, Generics};
use syntax::ast::{Item, ItemKind, ImplItem, ImplItemKind};
use syntax::ast::{Label, Local, Mutability, Pat, PatKind, Path};
use syntax::ast::{QSelf, TraitItemKind, TraitRef, Ty, TyKind};
use syntax::ptr::P;

use syntax_pos::{Span, DUMMY_SP, MultiSpan};
use errors::{Applicability, DiagnosticBuilder, DiagnosticId};

use std::cell::{Cell, RefCell};
use std::{cmp, fmt, iter, ptr};
use std::collections::BTreeSet;
use std::mem::replace;
use rustc_data_structures::ptr_key::PtrKey;
use rustc_data_structures::sync::Lrc;

use resolve_imports::{ImportDirective, ImportDirectiveSubclass, NameResolution, ImportResolver};
use macros::{InvocationData, LegacyBinding, ParentScope};

// N.B., this module needs to be declared first so diagnostics are
// registered before they are used.
mod diagnostics;
mod error_reporting;
mod macros;
mod check_unused;
mod build_reduced_graph;
mod resolve_imports;

fn is_known_tool(name: Name) -> bool {
    ["clippy", "rustfmt"].contains(&&*name.as_str())
}

enum Weak {
    Yes,
    No,
}

enum ScopeSet {
    Import(Namespace),
    AbsolutePath(Namespace),
    Macro(MacroKind),
    Module,
}

/// A free importable items suggested in case of resolution failure.
struct ImportSuggestion {
    path: Path,
}

/// A field or associated item from self type suggested in case of resolution failure.
enum AssocSuggestion {
    Field,
    MethodWithSelf,
    AssocItem,
}

#[derive(Eq)]
struct BindingError {
    name: Name,
    origin: BTreeSet<Span>,
    target: BTreeSet<Span>,
}

impl PartialOrd for BindingError {
    fn partial_cmp(&self, other: &BindingError) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BindingError {
    fn eq(&self, other: &BindingError) -> bool {
        self.name == other.name
    }
}

impl Ord for BindingError {
    fn cmp(&self, other: &BindingError) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

enum ResolutionError<'a> {
    /// error E0401: can't use type parameters from outer function
    TypeParametersFromOuterFunction(Def),
    /// error E0403: the name is already used for a type parameter in this type parameter list
    NameAlreadyUsedInTypeParameterList(Name, &'a Span),
    /// error E0407: method is not a member of trait
    MethodNotMemberOfTrait(Name, &'a str),
    /// error E0437: type is not a member of trait
    TypeNotMemberOfTrait(Name, &'a str),
    /// error E0438: const is not a member of trait
    ConstNotMemberOfTrait(Name, &'a str),
    /// error E0408: variable `{}` is not bound in all patterns
    VariableNotBoundInPattern(&'a BindingError),
    /// error E0409: variable `{}` is bound in inconsistent ways within the same match arm
    VariableBoundWithDifferentMode(Name, Span),
    /// error E0415: identifier is bound more than once in this parameter list
    IdentifierBoundMoreThanOnceInParameterList(&'a str),
    /// error E0416: identifier is bound more than once in the same pattern
    IdentifierBoundMoreThanOnceInSamePattern(&'a str),
    /// error E0426: use of undeclared label
    UndeclaredLabel(&'a str, Option<Name>),
    /// error E0429: `self` imports are only allowed within a { } list
    SelfImportsOnlyAllowedWithin,
    /// error E0430: `self` import can only appear once in the list
    SelfImportCanOnlyAppearOnceInTheList,
    /// error E0431: `self` import can only appear in an import list with a non-empty prefix
    SelfImportOnlyInImportListWithNonEmptyPrefix,
    /// error E0433: failed to resolve
    FailedToResolve(&'a str),
    /// error E0434: can't capture dynamic environment in a fn item
    CannotCaptureDynamicEnvironmentInFnItem,
    /// error E0435: attempt to use a non-constant value in a constant
    AttemptToUseNonConstantValueInConstant,
    /// error E0530: X bindings cannot shadow Ys
    BindingShadowsSomethingUnacceptable(&'a str, Name, &'a NameBinding<'a>),
    /// error E0128: type parameters with a default cannot use forward declared identifiers
    ForwardDeclaredTyParam,
}

/// Combines an error with provided span and emits it
///
/// This takes the error provided, combines it with the span and any additional spans inside the
/// error and emits it.
fn resolve_error<'sess, 'a>(resolver: &'sess Resolver,
                            span: Span,
                            resolution_error: ResolutionError<'a>) {
    resolve_struct_error(resolver, span, resolution_error).emit();
}

fn resolve_struct_error<'sess, 'a>(resolver: &'sess Resolver,
                                   span: Span,
                                   resolution_error: ResolutionError<'a>)
                                   -> DiagnosticBuilder<'sess> {
    match resolution_error {
        ResolutionError::TypeParametersFromOuterFunction(outer_def) => {
            let mut err = struct_span_err!(resolver.session,
                                           span,
                                           E0401,
                                           "can't use type parameters from outer function");
            err.span_label(span, "use of type variable from outer function");

            let cm = resolver.session.source_map();
            match outer_def {
                Def::SelfTy(maybe_trait_defid, maybe_impl_defid) => {
                    if let Some(impl_span) = maybe_impl_defid.and_then(|def_id| {
                        resolver.definitions.opt_span(def_id)
                    }) {
                        err.span_label(
                            reduce_impl_span_to_impl_keyword(cm, impl_span),
                            "`Self` type implicitly declared here, by this `impl`",
                        );
                    }
                    match (maybe_trait_defid, maybe_impl_defid) {
                        (Some(_), None) => {
                            err.span_label(span, "can't use `Self` here");
                        }
                        (_, Some(_)) => {
                            err.span_label(span, "use a type here instead");
                        }
                        (None, None) => bug!("`impl` without trait nor type?"),
                    }
                    return err;
                },
                Def::TyParam(typaram_defid) => {
                    if let Some(typaram_span) = resolver.definitions.opt_span(typaram_defid) {
                        err.span_label(typaram_span, "type variable from outer function");
                    }
                },
                _ => {
                    bug!("TypeParametersFromOuterFunction should only be used with Def::SelfTy or \
                         Def::TyParam")
                }
            }

            // Try to retrieve the span of the function signature and generate a new message with
            // a local type parameter
            let sugg_msg = "try using a local type parameter instead";
            if let Some((sugg_span, new_snippet)) = cm.generate_local_type_param_snippet(span) {
                // Suggest the modification to the user
                err.span_suggestion_with_applicability(
                    sugg_span,
                    sugg_msg,
                    new_snippet,
                    Applicability::MachineApplicable,
                );
            } else if let Some(sp) = cm.generate_fn_name_span(span) {
                err.span_label(sp, "try adding a local type parameter in this method instead");
            } else {
                err.help("try using a local type parameter instead");
            }

            err
        }
        ResolutionError::NameAlreadyUsedInTypeParameterList(name, first_use_span) => {
             let mut err = struct_span_err!(resolver.session,
                                            span,
                                            E0403,
                                            "the name `{}` is already used for a type parameter \
                                            in this type parameter list",
                                            name);
             err.span_label(span, "already used");
             err.span_label(first_use_span.clone(), format!("first use of `{}`", name));
             err
        }
        ResolutionError::MethodNotMemberOfTrait(method, trait_) => {
            let mut err = struct_span_err!(resolver.session,
                                           span,
                                           E0407,
                                           "method `{}` is not a member of trait `{}`",
                                           method,
                                           trait_);
            err.span_label(span, format!("not a member of trait `{}`", trait_));
            err
        }
        ResolutionError::TypeNotMemberOfTrait(type_, trait_) => {
            let mut err = struct_span_err!(resolver.session,
                             span,
                             E0437,
                             "type `{}` is not a member of trait `{}`",
                             type_,
                             trait_);
            err.span_label(span, format!("not a member of trait `{}`", trait_));
            err
        }
        ResolutionError::ConstNotMemberOfTrait(const_, trait_) => {
            let mut err = struct_span_err!(resolver.session,
                             span,
                             E0438,
                             "const `{}` is not a member of trait `{}`",
                             const_,
                             trait_);
            err.span_label(span, format!("not a member of trait `{}`", trait_));
            err
        }
        ResolutionError::VariableNotBoundInPattern(binding_error) => {
            let target_sp = binding_error.target.iter().cloned().collect::<Vec<_>>();
            let msp = MultiSpan::from_spans(target_sp.clone());
            let msg = format!("variable `{}` is not bound in all patterns", binding_error.name);
            let mut err = resolver.session.struct_span_err_with_code(
                msp,
                &msg,
                DiagnosticId::Error("E0408".into()),
            );
            for sp in target_sp {
                err.span_label(sp, format!("pattern doesn't bind `{}`", binding_error.name));
            }
            let origin_sp = binding_error.origin.iter().cloned();
            for sp in origin_sp {
                err.span_label(sp, "variable not in all patterns");
            }
            err
        }
        ResolutionError::VariableBoundWithDifferentMode(variable_name,
                                                        first_binding_span) => {
            let mut err = struct_span_err!(resolver.session,
                             span,
                             E0409,
                             "variable `{}` is bound in inconsistent \
                             ways within the same match arm",
                             variable_name);
            err.span_label(span, "bound in different ways");
            err.span_label(first_binding_span, "first binding");
            err
        }
        ResolutionError::IdentifierBoundMoreThanOnceInParameterList(identifier) => {
            let mut err = struct_span_err!(resolver.session,
                             span,
                             E0415,
                             "identifier `{}` is bound more than once in this parameter list",
                             identifier);
            err.span_label(span, "used as parameter more than once");
            err
        }
        ResolutionError::IdentifierBoundMoreThanOnceInSamePattern(identifier) => {
            let mut err = struct_span_err!(resolver.session,
                             span,
                             E0416,
                             "identifier `{}` is bound more than once in the same pattern",
                             identifier);
            err.span_label(span, "used in a pattern more than once");
            err
        }
        ResolutionError::UndeclaredLabel(name, lev_candidate) => {
            let mut err = struct_span_err!(resolver.session,
                                           span,
                                           E0426,
                                           "use of undeclared label `{}`",
                                           name);
            if let Some(lev_candidate) = lev_candidate {
                err.span_label(span, format!("did you mean `{}`?", lev_candidate));
            } else {
                err.span_label(span, format!("undeclared label `{}`", name));
            }
            err
        }
        ResolutionError::SelfImportsOnlyAllowedWithin => {
            struct_span_err!(resolver.session,
                             span,
                             E0429,
                             "{}",
                             "`self` imports are only allowed within a { } list")
        }
        ResolutionError::SelfImportCanOnlyAppearOnceInTheList => {
            let mut err = struct_span_err!(resolver.session, span, E0430,
                                           "`self` import can only appear once in an import list");
            err.span_label(span, "can only appear once in an import list");
            err
        }
        ResolutionError::SelfImportOnlyInImportListWithNonEmptyPrefix => {
            let mut err = struct_span_err!(resolver.session, span, E0431,
                                           "`self` import can only appear in an import list with \
                                            a non-empty prefix");
            err.span_label(span, "can only appear in an import list with a non-empty prefix");
            err
        }
        ResolutionError::FailedToResolve(msg) => {
            let mut err = struct_span_err!(resolver.session, span, E0433,
                                           "failed to resolve: {}", msg);
            err.span_label(span, msg);
            err
        }
        ResolutionError::CannotCaptureDynamicEnvironmentInFnItem => {
            let mut err = struct_span_err!(resolver.session,
                                           span,
                                           E0434,
                                           "{}",
                                           "can't capture dynamic environment in a fn item");
            err.help("use the `|| { ... }` closure form instead");
            err
        }
        ResolutionError::AttemptToUseNonConstantValueInConstant => {
            let mut err = struct_span_err!(resolver.session, span, E0435,
                                           "attempt to use a non-constant value in a constant");
            err.span_label(span, "non-constant value");
            err
        }
        ResolutionError::BindingShadowsSomethingUnacceptable(what_binding, name, binding) => {
            let shadows_what = binding.descr();
            let mut err = struct_span_err!(resolver.session, span, E0530, "{}s cannot shadow {}s",
                                           what_binding, shadows_what);
            err.span_label(span, format!("cannot be named the same as {} {}",
                                         binding.article(), shadows_what));
            let participle = if binding.is_import() { "imported" } else { "defined" };
            let msg = format!("the {} `{}` is {} here", shadows_what, name, participle);
            err.span_label(binding.span, msg);
            err
        }
        ResolutionError::ForwardDeclaredTyParam => {
            let mut err = struct_span_err!(resolver.session, span, E0128,
                                           "type parameters with a default cannot use \
                                            forward declared identifiers");
            err.span_label(
                span, "defaulted type parameters cannot be forward declared".to_string());
            err
        }
    }
}

/// Adjust the impl span so that just the `impl` keyword is taken by removing
/// everything after `<` (`"impl<T> Iterator for A<T> {}" -> "impl"`) and
/// everything after the first whitespace (`"impl Iterator for A" -> "impl"`)
///
/// Attention: The method used is very fragile since it essentially duplicates the work of the
/// parser. If you need to use this function or something similar, please consider updating the
/// source_map functions and this function to something more robust.
fn reduce_impl_span_to_impl_keyword(cm: &SourceMap, impl_span: Span) -> Span {
    let impl_span = cm.span_until_char(impl_span, '<');
    let impl_span = cm.span_until_whitespace(impl_span);
    impl_span
}

#[derive(Copy, Clone, Debug)]
struct BindingInfo {
    span: Span,
    binding_mode: BindingMode,
}

/// Map from the name in a pattern to its binding mode.
type BindingMap = FxHashMap<Ident, BindingInfo>;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum PatternSource {
    Match,
    IfLet,
    WhileLet,
    Let,
    For,
    FnParam,
}

impl PatternSource {
    fn descr(self) -> &'static str {
        match self {
            PatternSource::Match => "match binding",
            PatternSource::IfLet => "if let binding",
            PatternSource::WhileLet => "while let binding",
            PatternSource::Let => "let binding",
            PatternSource::For => "for binding",
            PatternSource::FnParam => "function parameter",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum AliasPossibility {
    No,
    Maybe,
}

#[derive(Copy, Clone, Debug)]
enum PathSource<'a> {
    // Type paths `Path`.
    Type,
    // Trait paths in bounds or impls.
    Trait(AliasPossibility),
    // Expression paths `path`, with optional parent context.
    Expr(Option<&'a Expr>),
    // Paths in path patterns `Path`.
    Pat,
    // Paths in struct expressions and patterns `Path { .. }`.
    Struct,
    // Paths in tuple struct patterns `Path(..)`.
    TupleStruct,
    // `m::A::B` in `<T as m::A>::B::C`.
    TraitItem(Namespace),
    // Path in `pub(path)`
    Visibility,
}

impl<'a> PathSource<'a> {
    fn namespace(self) -> Namespace {
        match self {
            PathSource::Type | PathSource::Trait(_) | PathSource::Struct |
            PathSource::Visibility => TypeNS,
            PathSource::Expr(..) | PathSource::Pat | PathSource::TupleStruct => ValueNS,
            PathSource::TraitItem(ns) => ns,
        }
    }

    fn global_by_default(self) -> bool {
        match self {
            PathSource::Visibility => true,
            PathSource::Type | PathSource::Expr(..) | PathSource::Pat |
            PathSource::Struct | PathSource::TupleStruct |
            PathSource::Trait(_) | PathSource::TraitItem(..) => false,
        }
    }

    fn defer_to_typeck(self) -> bool {
        match self {
            PathSource::Type | PathSource::Expr(..) | PathSource::Pat |
            PathSource::Struct | PathSource::TupleStruct => true,
            PathSource::Trait(_) | PathSource::TraitItem(..) |
            PathSource::Visibility => false,
        }
    }

    fn descr_expected(self) -> &'static str {
        match self {
            PathSource::Type => "type",
            PathSource::Trait(_) => "trait",
            PathSource::Pat => "unit struct/variant or constant",
            PathSource::Struct => "struct, variant or union type",
            PathSource::TupleStruct => "tuple struct/variant",
            PathSource::Visibility => "module",
            PathSource::TraitItem(ns) => match ns {
                TypeNS => "associated type",
                ValueNS => "method or associated constant",
                MacroNS => bug!("associated macro"),
            },
            PathSource::Expr(parent) => match parent.map(|p| &p.node) {
                // "function" here means "anything callable" rather than `Def::Fn`,
                // this is not precise but usually more helpful than just "value".
                Some(&ExprKind::Call(..)) => "function",
                _ => "value",
            },
        }
    }

    fn is_expected(self, def: Def) -> bool {
        match self {
            PathSource::Type => match def {
                Def::Struct(..) | Def::Union(..) | Def::Enum(..) |
                Def::Trait(..) | Def::TraitAlias(..) | Def::TyAlias(..) |
                Def::AssociatedTy(..) | Def::PrimTy(..) | Def::TyParam(..) |
                Def::SelfTy(..) | Def::Existential(..) |
                Def::ForeignTy(..) => true,
                _ => false,
            },
            PathSource::Trait(AliasPossibility::No) => match def {
                Def::Trait(..) => true,
                _ => false,
            },
            PathSource::Trait(AliasPossibility::Maybe) => match def {
                Def::Trait(..) => true,
                Def::TraitAlias(..) => true,
                _ => false,
            },
            PathSource::Expr(..) => match def {
                Def::StructCtor(_, CtorKind::Const) | Def::StructCtor(_, CtorKind::Fn) |
                Def::VariantCtor(_, CtorKind::Const) | Def::VariantCtor(_, CtorKind::Fn) |
                Def::Const(..) | Def::Static(..) | Def::Local(..) | Def::Upvar(..) |
                Def::Fn(..) | Def::Method(..) | Def::AssociatedConst(..) |
                Def::SelfCtor(..) => true,
                _ => false,
            },
            PathSource::Pat => match def {
                Def::StructCtor(_, CtorKind::Const) |
                Def::VariantCtor(_, CtorKind::Const) |
                Def::Const(..) | Def::AssociatedConst(..) |
                Def::SelfCtor(..) => true,
                _ => false,
            },
            PathSource::TupleStruct => match def {
                Def::StructCtor(_, CtorKind::Fn) |
                Def::VariantCtor(_, CtorKind::Fn) |
                Def::SelfCtor(..) => true,
                _ => false,
            },
            PathSource::Struct => match def {
                Def::Struct(..) | Def::Union(..) | Def::Variant(..) |
                Def::TyAlias(..) | Def::AssociatedTy(..) | Def::SelfTy(..) => true,
                _ => false,
            },
            PathSource::TraitItem(ns) => match def {
                Def::AssociatedConst(..) | Def::Method(..) if ns == ValueNS => true,
                Def::AssociatedTy(..) if ns == TypeNS => true,
                _ => false,
            },
            PathSource::Visibility => match def {
                Def::Mod(..) => true,
                _ => false,
            },
        }
    }

    fn error_code(self, has_unexpected_resolution: bool) -> &'static str {
        __diagnostic_used!(E0404);
        __diagnostic_used!(E0405);
        __diagnostic_used!(E0412);
        __diagnostic_used!(E0422);
        __diagnostic_used!(E0423);
        __diagnostic_used!(E0425);
        __diagnostic_used!(E0531);
        __diagnostic_used!(E0532);
        __diagnostic_used!(E0573);
        __diagnostic_used!(E0574);
        __diagnostic_used!(E0575);
        __diagnostic_used!(E0576);
        __diagnostic_used!(E0577);
        __diagnostic_used!(E0578);
        match (self, has_unexpected_resolution) {
            (PathSource::Trait(_), true) => "E0404",
            (PathSource::Trait(_), false) => "E0405",
            (PathSource::Type, true) => "E0573",
            (PathSource::Type, false) => "E0412",
            (PathSource::Struct, true) => "E0574",
            (PathSource::Struct, false) => "E0422",
            (PathSource::Expr(..), true) => "E0423",
            (PathSource::Expr(..), false) => "E0425",
            (PathSource::Pat, true) | (PathSource::TupleStruct, true) => "E0532",
            (PathSource::Pat, false) | (PathSource::TupleStruct, false) => "E0531",
            (PathSource::TraitItem(..), true) => "E0575",
            (PathSource::TraitItem(..), false) => "E0576",
            (PathSource::Visibility, true) => "E0577",
            (PathSource::Visibility, false) => "E0578",
        }
    }
}

// A minimal representation of a path segment. We use this in resolve because
// we synthesize 'path segments' which don't have the rest of an AST or HIR
// `PathSegment`.
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    ident: Ident,
    id: Option<NodeId>,
}

impl Segment {
    fn from_path(path: &Path) -> Vec<Segment> {
        path.segments.iter().map(|s| s.into()).collect()
    }

    fn from_ident(ident: Ident) -> Segment {
        Segment {
            ident,
            id: None,
        }
    }

    fn names_to_string(segments: &[Segment]) -> String {
        names_to_string(&segments.iter()
                            .map(|seg| seg.ident)
                            .collect::<Vec<_>>())
    }
}

impl<'a> From<&'a ast::PathSegment> for Segment {
    fn from(seg: &'a ast::PathSegment) -> Segment {
        Segment {
            ident: seg.ident,
            id: Some(seg.id),
        }
    }
}

struct UsePlacementFinder {
    target_module: NodeId,
    span: Option<Span>,
    found_use: bool,
}

impl UsePlacementFinder {
    fn check(krate: &Crate, target_module: NodeId) -> (Option<Span>, bool) {
        let mut finder = UsePlacementFinder {
            target_module,
            span: None,
            found_use: false,
        };
        visit::walk_crate(&mut finder, krate);
        (finder.span, finder.found_use)
    }
}

impl<'tcx> Visitor<'tcx> for UsePlacementFinder {
    fn visit_mod(
        &mut self,
        module: &'tcx ast::Mod,
        _: Span,
        _: &[ast::Attribute],
        node_id: NodeId,
    ) {
        if self.span.is_some() {
            return;
        }
        if node_id != self.target_module {
            visit::walk_mod(self, module);
            return;
        }
        // find a use statement
        for item in &module.items {
            match item.node {
                ItemKind::Use(..) => {
                    // don't suggest placing a use before the prelude
                    // import or other generated ones
                    if item.span.ctxt().outer().expn_info().is_none() {
                        self.span = Some(item.span.shrink_to_lo());
                        self.found_use = true;
                        return;
                    }
                },
                // don't place use before extern crate
                ItemKind::ExternCrate(_) => {}
                // but place them before the first other item
                _ => if self.span.map_or(true, |span| item.span < span ) {
                    if item.span.ctxt().outer().expn_info().is_none() {
                        // don't insert between attributes and an item
                        if item.attrs.is_empty() {
                            self.span = Some(item.span.shrink_to_lo());
                        } else {
                            // find the first attribute on the item
                            for attr in &item.attrs {
                                if self.span.map_or(true, |span| attr.span < span) {
                                    self.span = Some(attr.span.shrink_to_lo());
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

/// This thing walks the whole crate in DFS manner, visiting each item, resolving names as it goes.
impl<'a, 'tcx> Visitor<'tcx> for Resolver<'a> {
    fn visit_item(&mut self, item: &'tcx Item) {
        self.resolve_item(item);
    }
    fn visit_arm(&mut self, arm: &'tcx Arm) {
        self.resolve_arm(arm);
    }
    fn visit_block(&mut self, block: &'tcx Block) {
        self.resolve_block(block);
    }
    fn visit_anon_const(&mut self, constant: &'tcx ast::AnonConst) {
        self.with_constant_rib(|this| {
            visit::walk_anon_const(this, constant);
        });
    }
    fn visit_expr(&mut self, expr: &'tcx Expr) {
        self.resolve_expr(expr, None);
    }
    fn visit_local(&mut self, local: &'tcx Local) {
        self.resolve_local(local);
    }
    fn visit_ty(&mut self, ty: &'tcx Ty) {
        match ty.node {
            TyKind::Path(ref qself, ref path) => {
                self.smart_resolve_path(ty.id, qself.as_ref(), path, PathSource::Type);
            }
            TyKind::ImplicitSelf => {
                let self_ty = keywords::SelfUpper.ident();
                let def = self.resolve_ident_in_lexical_scope(self_ty, TypeNS, Some(ty.id), ty.span)
                              .map_or(Def::Err, |d| d.def());
                self.record_def(ty.id, PathResolution::new(def));
            }
            _ => (),
        }
        visit::walk_ty(self, ty);
    }
    fn visit_poly_trait_ref(&mut self,
                            tref: &'tcx ast::PolyTraitRef,
                            m: &'tcx ast::TraitBoundModifier) {
        self.smart_resolve_path(tref.trait_ref.ref_id, None,
                                &tref.trait_ref.path, PathSource::Trait(AliasPossibility::Maybe));
        visit::walk_poly_trait_ref(self, tref, m);
    }
    fn visit_foreign_item(&mut self, foreign_item: &'tcx ForeignItem) {
        let type_parameters = match foreign_item.node {
            ForeignItemKind::Fn(_, ref generics) => {
                HasTypeParameters(generics, ItemRibKind)
            }
            ForeignItemKind::Static(..) => NoTypeParameters,
            ForeignItemKind::Ty => NoTypeParameters,
            ForeignItemKind::Macro(..) => NoTypeParameters,
        };
        self.with_type_parameter_rib(type_parameters, |this| {
            visit::walk_foreign_item(this, foreign_item);
        });
    }
    fn visit_fn(&mut self,
                function_kind: FnKind<'tcx>,
                declaration: &'tcx FnDecl,
                _: Span,
                node_id: NodeId)
    {
        let (rib_kind, asyncness) = match function_kind {
            FnKind::ItemFn(_, ref header, ..) =>
                (ItemRibKind, header.asyncness),
            FnKind::Method(_, ref sig, _, _) =>
                (TraitOrImplItemRibKind, sig.header.asyncness),
            FnKind::Closure(_) =>
                // Async closures aren't resolved through `visit_fn`-- they're
                // processed separately
                (ClosureRibKind(node_id), IsAsync::NotAsync),
        };

        // Create a value rib for the function.
        self.ribs[ValueNS].push(Rib::new(rib_kind));

        // Create a label rib for the function.
        self.label_ribs.push(Rib::new(rib_kind));

        // Add each argument to the rib.
        let mut bindings_list = FxHashMap::default();
        for argument in &declaration.inputs {
            self.resolve_pattern(&argument.pat, PatternSource::FnParam, &mut bindings_list);

            self.visit_ty(&argument.ty);

            debug!("(resolving function) recorded argument");
        }
        visit::walk_fn_ret_ty(self, &declaration.output);

        // Resolve the function body, potentially inside the body of an async closure
        if let IsAsync::Async { closure_id, .. } = asyncness {
            let rib_kind = ClosureRibKind(closure_id);
            self.ribs[ValueNS].push(Rib::new(rib_kind));
            self.label_ribs.push(Rib::new(rib_kind));
        }

        match function_kind {
            FnKind::ItemFn(.., body) |
            FnKind::Method(.., body) => {
                self.visit_block(body);
            }
            FnKind::Closure(body) => {
                self.visit_expr(body);
            }
        };

        // Leave the body of the async closure
        if asyncness.is_async() {
            self.label_ribs.pop();
            self.ribs[ValueNS].pop();
        }

        debug!("(resolving function) leaving function");

        self.label_ribs.pop();
        self.ribs[ValueNS].pop();
    }
    fn visit_generics(&mut self, generics: &'tcx Generics) {
        // For type parameter defaults, we have to ban access
        // to following type parameters, as the Substs can only
        // provide previous type parameters as they're built. We
        // put all the parameters on the ban list and then remove
        // them one by one as they are processed and become available.
        let mut default_ban_rib = Rib::new(ForwardTyParamBanRibKind);
        let mut found_default = false;
        default_ban_rib.bindings.extend(generics.params.iter()
            .filter_map(|param| match param.kind {
                GenericParamKind::Lifetime { .. } => None,
                GenericParamKind::Type { ref default, .. } => {
                    found_default |= default.is_some();
                    if found_default {
                        Some((Ident::with_empty_ctxt(param.ident.name), Def::Err))
                    } else {
                        None
                    }
                }
            }));

        for param in &generics.params {
            match param.kind {
                GenericParamKind::Lifetime { .. } => self.visit_generic_param(param),
                GenericParamKind::Type { ref default, .. } => {
                    for bound in &param.bounds {
                        self.visit_param_bound(bound);
                    }

                    if let Some(ref ty) = default {
                        self.ribs[TypeNS].push(default_ban_rib);
                        self.visit_ty(ty);
                        default_ban_rib = self.ribs[TypeNS].pop().unwrap();
                    }

                    // Allow all following defaults to refer to this type parameter.
                    default_ban_rib.bindings.remove(&Ident::with_empty_ctxt(param.ident.name));
                }
            }
        }
        for p in &generics.where_clause.predicates {
            self.visit_where_predicate(p);
        }
    }
}

#[derive(Copy, Clone)]
enum TypeParameters<'a, 'b> {
    NoTypeParameters,
    HasTypeParameters(// Type parameters.
                      &'b Generics,

                      // The kind of the rib used for type parameters.
                      RibKind<'a>),
}

/// The rib kind controls the translation of local
/// definitions (`Def::Local`) to upvars (`Def::Upvar`).
#[derive(Copy, Clone, Debug)]
enum RibKind<'a> {
    /// No translation needs to be applied.
    NormalRibKind,

    /// We passed through a closure scope at the given node ID.
    /// Translate upvars as appropriate.
    ClosureRibKind(NodeId /* func id */),

    /// We passed through an impl or trait and are now in one of its
    /// methods or associated types. Allow references to ty params that impl or trait
    /// binds. Disallow any other upvars (including other ty params that are
    /// upvars).
    TraitOrImplItemRibKind,

    /// We passed through an item scope. Disallow upvars.
    ItemRibKind,

    /// We're in a constant item. Can't refer to dynamic stuff.
    ConstantItemRibKind,

    /// We passed through a module.
    ModuleRibKind(Module<'a>),

    /// We passed through a `macro_rules!` statement
    MacroDefinition(DefId),

    /// All bindings in this rib are type parameters that can't be used
    /// from the default of a type parameter because they're not declared
    /// before said type parameter. Also see the `visit_generics` override.
    ForwardTyParamBanRibKind,
}

/// One local scope.
///
/// A rib represents a scope names can live in. Note that these appear in many places, not just
/// around braces. At any place where the list of accessible names (of the given namespace)
/// changes or a new restrictions on the name accessibility are introduced, a new rib is put onto a
/// stack. This may be, for example, a `let` statement (because it introduces variables), a macro,
/// etc.
///
/// Different [rib kinds](enum.RibKind) are transparent for different names.
///
/// The resolution keeps a separate stack of ribs as it traverses the AST for each namespace. When
/// resolving, the name is looked up from inside out.
#[derive(Debug)]
struct Rib<'a> {
    bindings: FxHashMap<Ident, Def>,
    kind: RibKind<'a>,
}

impl<'a> Rib<'a> {
    fn new(kind: RibKind<'a>) -> Rib<'a> {
        Rib {
            bindings: Default::default(),
            kind,
        }
    }
}

/// An intermediate resolution result.
///
/// This refers to the thing referred by a name. The difference between `Def` and `Item` is that
/// items are visible in their whole block, while defs only from the place they are defined
/// forward.
enum LexicalScopeBinding<'a> {
    Item(&'a NameBinding<'a>),
    Def(Def),
}

impl<'a> LexicalScopeBinding<'a> {
    fn item(self) -> Option<&'a NameBinding<'a>> {
        match self {
            LexicalScopeBinding::Item(binding) => Some(binding),
            _ => None,
        }
    }

    fn def(self) -> Def {
        match self {
            LexicalScopeBinding::Item(binding) => binding.def(),
            LexicalScopeBinding::Def(def) => def,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum ModuleOrUniformRoot<'a> {
    /// Regular module.
    Module(Module<'a>),

    /// Virtual module that denotes resolution in crate root with fallback to extern prelude.
    CrateRootAndExternPrelude,

    /// Virtual module that denotes resolution in extern prelude.
    /// Used for paths starting with `::` on 2018 edition or `extern::`.
    ExternPrelude,

    /// Virtual module that denotes resolution in current scope.
    /// Used only for resolving single-segment imports. The reason it exists is that import paths
    /// are always split into two parts, the first of which should be some kind of module.
    CurrentScope,
}

impl ModuleOrUniformRoot<'_> {
    fn same_def(lhs: Self, rhs: Self) -> bool {
        match (lhs, rhs) {
            (ModuleOrUniformRoot::Module(lhs),
             ModuleOrUniformRoot::Module(rhs)) => lhs.def() == rhs.def(),
            (ModuleOrUniformRoot::CrateRootAndExternPrelude,
             ModuleOrUniformRoot::CrateRootAndExternPrelude) |
            (ModuleOrUniformRoot::ExternPrelude, ModuleOrUniformRoot::ExternPrelude) |
            (ModuleOrUniformRoot::CurrentScope, ModuleOrUniformRoot::CurrentScope) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
enum PathResult<'a> {
    Module(ModuleOrUniformRoot<'a>),
    NonModule(PathResolution),
    Indeterminate,
    Failed(Span, String, bool /* is the error from the last segment? */),
}

enum ModuleKind {
    /// An anonymous module, eg. just a block.
    ///
    /// ```
    /// fn main() {
    ///     fn f() {} // (1)
    ///     { // This is an anonymous module
    ///         f(); // This resolves to (2) as we are inside the block.
    ///         fn f() {} // (2)
    ///     }
    ///     f(); // Resolves to (1)
    /// }
    /// ```
    Block(NodeId),
    /// Any module with a name.
    ///
    /// This could be:
    ///
    /// * A normal module ‒ either `mod from_file;` or `mod from_block { }`.
    /// * A trait or an enum (it implicitly contains associated types, methods and variant
    ///   constructors).
    Def(Def, Name),
}

/// One node in the tree of modules.
pub struct ModuleData<'a> {
    parent: Option<Module<'a>>,
    kind: ModuleKind,

    // The def id of the closest normal module (`mod`) ancestor (including this module).
    normal_ancestor_id: DefId,

    resolutions: RefCell<FxHashMap<(Ident, Namespace), &'a RefCell<NameResolution<'a>>>>,
    single_segment_macro_resolutions: RefCell<Vec<(Ident, MacroKind, ParentScope<'a>,
                                                   Option<&'a NameBinding<'a>>)>>,
    multi_segment_macro_resolutions: RefCell<Vec<(Vec<Segment>, Span, MacroKind, ParentScope<'a>,
                                                  Option<Def>)>>,
    builtin_attrs: RefCell<Vec<(Ident, ParentScope<'a>)>>,

    // Macro invocations that can expand into items in this module.
    unresolved_invocations: RefCell<FxHashSet<Mark>>,

    no_implicit_prelude: bool,

    glob_importers: RefCell<Vec<&'a ImportDirective<'a>>>,
    globs: RefCell<Vec<&'a ImportDirective<'a>>>,

    // Used to memoize the traits in this module for faster searches through all traits in scope.
    traits: RefCell<Option<Box<[(Ident, &'a NameBinding<'a>)]>>>,

    // Whether this module is populated. If not populated, any attempt to
    // access the children must be preceded with a
    // `populate_module_if_necessary` call.
    populated: Cell<bool>,

    /// Span of the module itself. Used for error reporting.
    span: Span,

    expansion: Mark,
}

type Module<'a> = &'a ModuleData<'a>;

impl<'a> ModuleData<'a> {
    fn new(parent: Option<Module<'a>>,
           kind: ModuleKind,
           normal_ancestor_id: DefId,
           expansion: Mark,
           span: Span) -> Self {
        ModuleData {
            parent,
            kind,
            normal_ancestor_id,
            resolutions: Default::default(),
            single_segment_macro_resolutions: RefCell::new(Vec::new()),
            multi_segment_macro_resolutions: RefCell::new(Vec::new()),
            builtin_attrs: RefCell::new(Vec::new()),
            unresolved_invocations: Default::default(),
            no_implicit_prelude: false,
            glob_importers: RefCell::new(Vec::new()),
            globs: RefCell::new(Vec::new()),
            traits: RefCell::new(None),
            populated: Cell::new(normal_ancestor_id.is_local()),
            span,
            expansion,
        }
    }

    fn for_each_child<F: FnMut(Ident, Namespace, &'a NameBinding<'a>)>(&self, mut f: F) {
        for (&(ident, ns), name_resolution) in self.resolutions.borrow().iter() {
            name_resolution.borrow().binding.map(|binding| f(ident, ns, binding));
        }
    }

    fn for_each_child_stable<F: FnMut(Ident, Namespace, &'a NameBinding<'a>)>(&self, mut f: F) {
        let resolutions = self.resolutions.borrow();
        let mut resolutions = resolutions.iter().collect::<Vec<_>>();
        resolutions.sort_by_cached_key(|&(&(ident, ns), _)| (ident.as_str(), ns));
        for &(&(ident, ns), &resolution) in resolutions.iter() {
            resolution.borrow().binding.map(|binding| f(ident, ns, binding));
        }
    }

    fn def(&self) -> Option<Def> {
        match self.kind {
            ModuleKind::Def(def, _) => Some(def),
            _ => None,
        }
    }

    fn def_id(&self) -> Option<DefId> {
        self.def().as_ref().map(Def::def_id)
    }

    // `self` resolves to the first module ancestor that `is_normal`.
    fn is_normal(&self) -> bool {
        match self.kind {
            ModuleKind::Def(Def::Mod(_), _) => true,
            _ => false,
        }
    }

    fn is_trait(&self) -> bool {
        match self.kind {
            ModuleKind::Def(Def::Trait(_), _) => true,
            _ => false,
        }
    }

    fn nearest_item_scope(&'a self) -> Module<'a> {
        if self.is_trait() { self.parent.unwrap() } else { self }
    }

    fn is_ancestor_of(&self, mut other: &Self) -> bool {
        while !ptr::eq(self, other) {
            if let Some(parent) = other.parent {
                other = parent;
            } else {
                return false;
            }
        }
        true
    }
}

impl<'a> fmt::Debug for ModuleData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.def())
    }
}

/// Records a possibly-private value, type, or module definition.
#[derive(Clone, Debug)]
pub struct NameBinding<'a> {
    kind: NameBindingKind<'a>,
    ambiguity: Option<(&'a NameBinding<'a>, AmbiguityKind)>,
    expansion: Mark,
    span: Span,
    vis: ty::Visibility,
}

pub trait ToNameBinding<'a> {
    fn to_name_binding(self, arenas: &'a ResolverArenas<'a>) -> &'a NameBinding<'a>;
}

impl<'a> ToNameBinding<'a> for &'a NameBinding<'a> {
    fn to_name_binding(self, _: &'a ResolverArenas<'a>) -> &'a NameBinding<'a> {
        self
    }
}

#[derive(Clone, Debug)]
enum NameBindingKind<'a> {
    Def(Def, /* is_macro_export */ bool),
    Module(Module<'a>),
    Import {
        binding: &'a NameBinding<'a>,
        directive: &'a ImportDirective<'a>,
        used: Cell<bool>,
    },
}

struct PrivacyError<'a>(Span, Ident, &'a NameBinding<'a>);

struct UseError<'a> {
    err: DiagnosticBuilder<'a>,
    /// Attach `use` statements for these candidates
    candidates: Vec<ImportSuggestion>,
    /// The node id of the module to place the use statements in
    node_id: NodeId,
    /// Whether the diagnostic should state that it's "better"
    better: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum AmbiguityKind {
    Import,
    AbsolutePath,
    BuiltinAttr,
    DeriveHelper,
    LegacyHelperVsPrelude,
    LegacyVsModern,
    GlobVsOuter,
    GlobVsGlob,
    GlobVsExpanded,
    MoreExpandedVsOuter,
}

impl AmbiguityKind {
    fn descr(self) -> &'static str {
        match self {
            AmbiguityKind::Import =>
                "name vs any other name during import resolution",
            AmbiguityKind::AbsolutePath =>
                "name in the crate root vs extern crate during absolute path resolution",
            AmbiguityKind::BuiltinAttr =>
                "built-in attribute vs any other name",
            AmbiguityKind::DeriveHelper =>
                "derive helper attribute vs any other name",
            AmbiguityKind::LegacyHelperVsPrelude =>
                "legacy plugin helper attribute vs name from prelude",
            AmbiguityKind::LegacyVsModern =>
                "`macro_rules` vs non-`macro_rules` from other module",
            AmbiguityKind::GlobVsOuter =>
                "glob import vs any other name from outer scope during import/macro resolution",
            AmbiguityKind::GlobVsGlob =>
                "glob import vs glob import in the same module",
            AmbiguityKind::GlobVsExpanded =>
                "glob import vs macro-expanded name in the same \
                 module during import/macro resolution",
            AmbiguityKind::MoreExpandedVsOuter =>
                "macro-expanded name vs less macro-expanded name \
                 from outer scope during import/macro resolution",
        }
    }
}

/// Miscellaneous bits of metadata for better ambiguity error reporting.
#[derive(Clone, Copy, PartialEq)]
enum AmbiguityErrorMisc {
    SuggestCrate,
    SuggestSelf,
    FromPrelude,
    None,
}

struct AmbiguityError<'a> {
    kind: AmbiguityKind,
    ident: Ident,
    b1: &'a NameBinding<'a>,
    b2: &'a NameBinding<'a>,
    misc1: AmbiguityErrorMisc,
    misc2: AmbiguityErrorMisc,
}

impl<'a> NameBinding<'a> {
    fn module(&self) -> Option<Module<'a>> {
        match self.kind {
            NameBindingKind::Module(module) => Some(module),
            NameBindingKind::Import { binding, .. } => binding.module(),
            _ => None,
        }
    }

    fn def(&self) -> Def {
        match self.kind {
            NameBindingKind::Def(def, _) => def,
            NameBindingKind::Module(module) => module.def().unwrap(),
            NameBindingKind::Import { binding, .. } => binding.def(),
        }
    }

    fn is_ambiguity(&self) -> bool {
        self.ambiguity.is_some() || match self.kind {
            NameBindingKind::Import { binding, .. } => binding.is_ambiguity(),
            _ => false,
        }
    }

    // We sometimes need to treat variants as `pub` for backwards compatibility
    fn pseudo_vis(&self) -> ty::Visibility {
        if self.is_variant() && self.def().def_id().is_local() {
            ty::Visibility::Public
        } else {
            self.vis
        }
    }

    fn is_variant(&self) -> bool {
        match self.kind {
            NameBindingKind::Def(Def::Variant(..), _) |
            NameBindingKind::Def(Def::VariantCtor(..), _) => true,
            _ => false,
        }
    }

    fn is_extern_crate(&self) -> bool {
        match self.kind {
            NameBindingKind::Import {
                directive: &ImportDirective {
                    subclass: ImportDirectiveSubclass::ExternCrate { .. }, ..
                }, ..
            } => true,
            NameBindingKind::Module(
                &ModuleData { kind: ModuleKind::Def(Def::Mod(def_id), _), .. }
            ) => def_id.index == CRATE_DEF_INDEX,
            _ => false,
        }
    }

    fn is_import(&self) -> bool {
        match self.kind {
            NameBindingKind::Import { .. } => true,
            _ => false,
        }
    }

    fn is_glob_import(&self) -> bool {
        match self.kind {
            NameBindingKind::Import { directive, .. } => directive.is_glob(),
            _ => false,
        }
    }

    fn is_importable(&self) -> bool {
        match self.def() {
            Def::AssociatedConst(..) | Def::Method(..) | Def::AssociatedTy(..) => false,
            _ => true,
        }
    }

    fn is_macro_def(&self) -> bool {
        match self.kind {
            NameBindingKind::Def(Def::Macro(..), _) => true,
            _ => false,
        }
    }

    fn macro_kind(&self) -> Option<MacroKind> {
        match self.def() {
            Def::Macro(_, kind) => Some(kind),
            Def::NonMacroAttr(..) => Some(MacroKind::Attr),
            _ => None,
        }
    }

    fn descr(&self) -> &'static str {
        if self.is_extern_crate() { "extern crate" } else { self.def().kind_name() }
    }

    fn article(&self) -> &'static str {
        if self.is_extern_crate() { "an" } else { self.def().article() }
    }

    // Suppose that we resolved macro invocation with `invoc_parent_expansion` to binding `binding`
    // at some expansion round `max(invoc, binding)` when they both emerged from macros.
    // Then this function returns `true` if `self` may emerge from a macro *after* that
    // in some later round and screw up our previously found resolution.
    // See more detailed explanation in
    // https://github.com/rust-lang/rust/pull/53778#issuecomment-419224049
    fn may_appear_after(&self, invoc_parent_expansion: Mark, binding: &NameBinding) -> bool {
        // self > max(invoc, binding) => !(self <= invoc || self <= binding)
        // Expansions are partially ordered, so "may appear after" is an inversion of
        // "certainly appears before or simultaneously" and includes unordered cases.
        let self_parent_expansion = self.expansion;
        let other_parent_expansion = binding.expansion;
        let certainly_before_other_or_simultaneously =
            other_parent_expansion.is_descendant_of(self_parent_expansion);
        let certainly_before_invoc_or_simultaneously =
            invoc_parent_expansion.is_descendant_of(self_parent_expansion);
        !(certainly_before_other_or_simultaneously || certainly_before_invoc_or_simultaneously)
    }
}

/// Interns the names of the primitive types.
///
/// All other types are defined somewhere and possibly imported, but the primitive ones need
/// special handling, since they have no place of origin.
#[derive(Default)]
struct PrimitiveTypeTable {
    primitive_types: FxHashMap<Name, PrimTy>,
}

impl PrimitiveTypeTable {
    fn new() -> PrimitiveTypeTable {
        let mut table = PrimitiveTypeTable::default();

        table.intern("bool", Bool);
        table.intern("char", Char);
        table.intern("f32", Float(FloatTy::F32));
        table.intern("f64", Float(FloatTy::F64));
        table.intern("isize", Int(IntTy::Isize));
        table.intern("i8", Int(IntTy::I8));
        table.intern("i16", Int(IntTy::I16));
        table.intern("i32", Int(IntTy::I32));
        table.intern("i64", Int(IntTy::I64));
        table.intern("i128", Int(IntTy::I128));
        table.intern("str", Str);
        table.intern("usize", Uint(UintTy::Usize));
        table.intern("u8", Uint(UintTy::U8));
        table.intern("u16", Uint(UintTy::U16));
        table.intern("u32", Uint(UintTy::U32));
        table.intern("u64", Uint(UintTy::U64));
        table.intern("u128", Uint(UintTy::U128));
        table
    }

    fn intern(&mut self, string: &str, primitive_type: PrimTy) {
        self.primitive_types.insert(Symbol::intern(string), primitive_type);
    }
}

#[derive(Default, Clone)]
pub struct ExternPreludeEntry<'a> {
    extern_crate_item: Option<&'a NameBinding<'a>>,
    pub introduced_by_item: bool,
}

/// The main resolver class.
///
/// This is the visitor that walks the whole crate.
pub struct Resolver<'a> {
    session: &'a Session,
    cstore: &'a CStore,

    pub definitions: Definitions,

    graph_root: Module<'a>,

    prelude: Option<Module<'a>>,
    pub extern_prelude: FxHashMap<Ident, ExternPreludeEntry<'a>>,

    /// n.b. This is used only for better diagnostics, not name resolution itself.
    has_self: FxHashSet<DefId>,

    /// Names of fields of an item `DefId` accessible with dot syntax.
    /// Used for hints during error reporting.
    field_names: FxHashMap<DefId, Vec<Name>>,

    /// All imports known to succeed or fail.
    determined_imports: Vec<&'a ImportDirective<'a>>,

    /// All non-determined imports.
    indeterminate_imports: Vec<&'a ImportDirective<'a>>,

    /// The module that represents the current item scope.
    current_module: Module<'a>,

    /// The current set of local scopes for types and values.
    /// FIXME #4948: Reuse ribs to avoid allocation.
    ribs: PerNS<Vec<Rib<'a>>>,

    /// The current set of local scopes, for labels.
    label_ribs: Vec<Rib<'a>>,

    /// The trait that the current context can refer to.
    current_trait_ref: Option<(Module<'a>, TraitRef)>,

    /// The current self type if inside an impl (used for better errors).
    current_self_type: Option<Ty>,

    /// The current self item if inside an ADT (used for better errors).
    current_self_item: Option<NodeId>,

    /// FIXME: Refactor things so that these fields are passed through arguments and not resolver.
    /// We are resolving a last import segment during import validation.
    last_import_segment: bool,
    /// This binding should be ignored during in-module resolution, so that we don't get
    /// "self-confirming" import resolutions during import validation.
    blacklisted_binding: Option<&'a NameBinding<'a>>,

    /// The idents for the primitive types.
    primitive_type_table: PrimitiveTypeTable,

    def_map: DefMap,
    import_map: ImportMap,
    pub freevars: FreevarMap,
    freevars_seen: NodeMap<NodeMap<usize>>,
    pub export_map: ExportMap,
    pub trait_map: TraitMap,

    /// A map from nodes to anonymous modules.
    /// Anonymous modules are pseudo-modules that are implicitly created around items
    /// contained within blocks.
    ///
    /// For example, if we have this:
    ///
    ///  fn f() {
    ///      fn g() {
    ///          ...
    ///      }
    ///  }
    ///
    /// There will be an anonymous module created around `g` with the ID of the
    /// entry block for `f`.
    block_map: NodeMap<Module<'a>>,
    module_map: FxHashMap<DefId, Module<'a>>,
    extern_module_map: FxHashMap<(DefId, bool /* MacrosOnly? */), Module<'a>>,
    binding_parent_modules: FxHashMap<PtrKey<'a, NameBinding<'a>>, Module<'a>>,

    pub make_glob_map: bool,
    /// Maps imports to the names of items actually imported (this actually maps
    /// all imports, but only glob imports are actually interesting).
    pub glob_map: GlobMap,

    used_imports: FxHashSet<(NodeId, Namespace)>,
    pub maybe_unused_trait_imports: NodeSet,
    pub maybe_unused_extern_crates: Vec<(NodeId, Span)>,

    /// A list of labels as of yet unused. Labels will be removed from this map when
    /// they are used (in a `break` or `continue` statement)
    pub unused_labels: FxHashMap<NodeId, Span>,

    /// privacy errors are delayed until the end in order to deduplicate them
    privacy_errors: Vec<PrivacyError<'a>>,
    /// ambiguity errors are delayed for deduplication
    ambiguity_errors: Vec<AmbiguityError<'a>>,
    /// `use` injections are delayed for better placement and deduplication
    use_injections: Vec<UseError<'a>>,
    /// crate-local macro expanded `macro_export` referred to by a module-relative path
    macro_expanded_macro_export_errors: BTreeSet<(Span, Span)>,

    arenas: &'a ResolverArenas<'a>,
    dummy_binding: &'a NameBinding<'a>,

    crate_loader: &'a mut CrateLoader<'a>,
    macro_names: FxHashSet<Ident>,
    builtin_macros: FxHashMap<Name, &'a NameBinding<'a>>,
    macro_use_prelude: FxHashMap<Name, &'a NameBinding<'a>>,
    pub all_macros: FxHashMap<Name, Def>,
    macro_map: FxHashMap<DefId, Lrc<SyntaxExtension>>,
    macro_defs: FxHashMap<Mark, DefId>,
    local_macro_def_scopes: FxHashMap<NodeId, Module<'a>>,

    /// List of crate local macros that we need to warn about as being unused.
    /// Right now this only includes macro_rules! macros, and macros 2.0.
    unused_macros: FxHashSet<DefId>,

    /// Maps the `Mark` of an expansion to its containing module or block.
    invocations: FxHashMap<Mark, &'a InvocationData<'a>>,

    /// Avoid duplicated errors for "name already defined".
    name_already_seen: FxHashMap<Name, Span>,

    potentially_unused_imports: Vec<&'a ImportDirective<'a>>,

    /// This table maps struct IDs into struct constructor IDs,
    /// it's not used during normal resolution, only for better error reporting.
    struct_constructors: DefIdMap<(Def, ty::Visibility)>,

    /// Only used for better errors on `fn(): fn()`
    current_type_ascription: Vec<Span>,

    injected_crate: Option<Module<'a>>,
}

/// Nothing really interesting here, it just provides memory for the rest of the crate.
#[derive(Default)]
pub struct ResolverArenas<'a> {
    modules: arena::TypedArena<ModuleData<'a>>,
    local_modules: RefCell<Vec<Module<'a>>>,
    name_bindings: arena::TypedArena<NameBinding<'a>>,
    import_directives: arena::TypedArena<ImportDirective<'a>>,
    name_resolutions: arena::TypedArena<RefCell<NameResolution<'a>>>,
    invocation_data: arena::TypedArena<InvocationData<'a>>,
    legacy_bindings: arena::TypedArena<LegacyBinding<'a>>,
}

impl<'a> ResolverArenas<'a> {
    fn alloc_module(&'a self, module: ModuleData<'a>) -> Module<'a> {
        let module = self.modules.alloc(module);
        if module.def_id().map(|def_id| def_id.is_local()).unwrap_or(true) {
            self.local_modules.borrow_mut().push(module);
        }
        module
    }
    fn local_modules(&'a self) -> ::std::cell::Ref<'a, Vec<Module<'a>>> {
        self.local_modules.borrow()
    }
    fn alloc_name_binding(&'a self, name_binding: NameBinding<'a>) -> &'a NameBinding<'a> {
        self.name_bindings.alloc(name_binding)
    }
    fn alloc_import_directive(&'a self, import_directive: ImportDirective<'a>)
                              -> &'a ImportDirective {
        self.import_directives.alloc(import_directive)
    }
    fn alloc_name_resolution(&'a self) -> &'a RefCell<NameResolution<'a>> {
        self.name_resolutions.alloc(Default::default())
    }
    fn alloc_invocation_data(&'a self, expansion_data: InvocationData<'a>)
                             -> &'a InvocationData<'a> {
        self.invocation_data.alloc(expansion_data)
    }
    fn alloc_legacy_binding(&'a self, binding: LegacyBinding<'a>) -> &'a LegacyBinding<'a> {
        self.legacy_bindings.alloc(binding)
    }
}

impl<'a, 'b: 'a> ty::DefIdTree for &'a Resolver<'b> {
    fn parent(self, id: DefId) -> Option<DefId> {
        match id.krate {
            LOCAL_CRATE => self.definitions.def_key(id.index).parent,
            _ => self.cstore.def_key(id).parent,
        }.map(|index| DefId { index, ..id })
    }
}

/// This interface is used through the AST→HIR step, to embed full paths into the HIR. After that
/// the resolver is no longer needed as all the relevant information is inline.
impl<'a> hir::lowering::Resolver for Resolver<'a> {
    fn resolve_hir_path(
        &mut self,
        path: &ast::Path,
        is_value: bool,
    ) -> hir::Path {
        self.resolve_hir_path_cb(path, is_value,
                                 |resolver, span, error| resolve_error(resolver, span, error))
    }

    fn resolve_str_path(
        &mut self,
        span: Span,
        crate_root: Option<&str>,
        components: &[&str],
        is_value: bool
    ) -> hir::Path {
        let segments = iter::once(keywords::PathRoot.ident())
            .chain(
                crate_root.into_iter()
                    .chain(components.iter().cloned())
                    .map(Ident::from_str)
            ).map(|i| self.new_ast_path_segment(i)).collect::<Vec<_>>();


        let path = ast::Path {
            span,
            segments,
        };

        self.resolve_hir_path(&path, is_value)
    }

    fn get_resolution(&mut self, id: NodeId) -> Option<PathResolution> {
        self.def_map.get(&id).cloned()
    }

    fn get_import(&mut self, id: NodeId) -> PerNS<Option<PathResolution>> {
        self.import_map.get(&id).cloned().unwrap_or_default()
    }

    fn definitions(&mut self) -> &mut Definitions {
        &mut self.definitions
    }
}

impl<'a> Resolver<'a> {
    /// Rustdoc uses this to resolve things in a recoverable way. ResolutionError<'a>
    /// isn't something that can be returned because it can't be made to live that long,
    /// and also it's a private type. Fortunately rustdoc doesn't need to know the error,
    /// just that an error occurred.
    pub fn resolve_str_path_error(&mut self, span: Span, path_str: &str, is_value: bool)
        -> Result<hir::Path, ()> {
        use std::iter;
        let mut errored = false;

        let path = if path_str.starts_with("::") {
            ast::Path {
                span,
                segments: iter::once(keywords::PathRoot.ident())
                    .chain({
                        path_str.split("::").skip(1).map(Ident::from_str)
                    })
                    .map(|i| self.new_ast_path_segment(i))
                    .collect(),
            }
        } else {
            ast::Path {
                span,
                segments: path_str
                    .split("::")
                    .map(Ident::from_str)
                    .map(|i| self.new_ast_path_segment(i))
                    .collect(),
            }
        };
        let path = self.resolve_hir_path_cb(&path, is_value, |_, _, _| errored = true);
        if errored || path.def == Def::Err {
            Err(())
        } else {
            Ok(path)
        }
    }

    /// resolve_hir_path, but takes a callback in case there was an error
    fn resolve_hir_path_cb<F>(
        &mut self,
        path: &ast::Path,
        is_value: bool,
        error_callback: F,
    ) -> hir::Path
        where F: for<'c, 'b> FnOnce(&'c mut Resolver, Span, ResolutionError<'b>)
    {
        let namespace = if is_value { ValueNS } else { TypeNS };
        let span = path.span;
        let segments = &path.segments;
        let path = Segment::from_path(&path);
        // FIXME (Manishearth): Intra doc links won't get warned of epoch changes
        let def = match self.resolve_path_without_parent_scope(&path, Some(namespace), true,
                                                               span, CrateLint::No) {
            PathResult::Module(ModuleOrUniformRoot::Module(module)) =>
                module.def().unwrap(),
            PathResult::NonModule(path_res) if path_res.unresolved_segments() == 0 =>
                path_res.base_def(),
            PathResult::NonModule(..) => {
                let msg = "type-relative paths are not supported in this context";
                error_callback(self, span, ResolutionError::FailedToResolve(msg));
                Def::Err
            }
            PathResult::Module(..) | PathResult::Indeterminate => unreachable!(),
            PathResult::Failed(span, msg, _) => {
                error_callback(self, span, ResolutionError::FailedToResolve(&msg));
                Def::Err
            }
        };

        let segments: Vec<_> = segments.iter().map(|seg| {
            let mut hir_seg = hir::PathSegment::from_ident(seg.ident);
            hir_seg.def = Some(self.def_map.get(&seg.id).map_or(Def::Err, |p| p.base_def()));
            hir_seg
        }).collect();
        hir::Path {
            span,
            def,
            segments: segments.into(),
        }
    }

    fn new_ast_path_segment(&self, ident: Ident) -> ast::PathSegment {
        let mut seg = ast::PathSegment::from_ident(ident);
        seg.id = self.session.next_node_id();
        seg
    }
}

impl<'a> Resolver<'a> {
    pub fn new(session: &'a Session,
               cstore: &'a CStore,
               krate: &Crate,
               crate_name: &str,
               make_glob_map: MakeGlobMap,
               crate_loader: &'a mut CrateLoader<'a>,
               arenas: &'a ResolverArenas<'a>)
               -> Resolver<'a> {
        let root_def_id = DefId::local(CRATE_DEF_INDEX);
        let root_module_kind = ModuleKind::Def(Def::Mod(root_def_id), keywords::Invalid.name());
        let graph_root = arenas.alloc_module(ModuleData {
            no_implicit_prelude: attr::contains_name(&krate.attrs, "no_implicit_prelude"),
            ..ModuleData::new(None, root_module_kind, root_def_id, Mark::root(), krate.span)
        });
        let mut module_map = FxHashMap::default();
        module_map.insert(DefId::local(CRATE_DEF_INDEX), graph_root);

        let mut definitions = Definitions::new();
        DefCollector::new(&mut definitions, Mark::root())
            .collect_root(crate_name, session.local_crate_disambiguator());

        let mut extern_prelude: FxHashMap<Ident, ExternPreludeEntry> =
            session.opts.externs.iter().map(|kv| (Ident::from_str(kv.0), Default::default()))
                                       .collect();

        if !attr::contains_name(&krate.attrs, "no_core") {
            extern_prelude.insert(Ident::from_str("core"), Default::default());
            if !attr::contains_name(&krate.attrs, "no_std") {
                extern_prelude.insert(Ident::from_str("std"), Default::default());
                if session.rust_2018() {
                    extern_prelude.insert(Ident::from_str("meta"), Default::default());
                }
            }
        }

        let mut invocations = FxHashMap::default();
        invocations.insert(Mark::root(),
                           arenas.alloc_invocation_data(InvocationData::root(graph_root)));

        let mut macro_defs = FxHashMap::default();
        macro_defs.insert(Mark::root(), root_def_id);

        Resolver {
            session,

            cstore,

            definitions,

            // The outermost module has def ID 0; this is not reflected in the
            // AST.
            graph_root,
            prelude: None,
            extern_prelude,

            has_self: FxHashSet::default(),
            field_names: FxHashMap::default(),

            determined_imports: Vec::new(),
            indeterminate_imports: Vec::new(),

            current_module: graph_root,
            ribs: PerNS {
                value_ns: vec![Rib::new(ModuleRibKind(graph_root))],
                type_ns: vec![Rib::new(ModuleRibKind(graph_root))],
                macro_ns: vec![Rib::new(ModuleRibKind(graph_root))],
            },
            label_ribs: Vec::new(),

            current_trait_ref: None,
            current_self_type: None,
            current_self_item: None,
            last_import_segment: false,
            blacklisted_binding: None,

            primitive_type_table: PrimitiveTypeTable::new(),

            def_map: Default::default(),
            import_map: Default::default(),
            freevars: Default::default(),
            freevars_seen: Default::default(),
            export_map: FxHashMap::default(),
            trait_map: Default::default(),
            module_map,
            block_map: Default::default(),
            extern_module_map: FxHashMap::default(),
            binding_parent_modules: FxHashMap::default(),

            make_glob_map: make_glob_map == MakeGlobMap::Yes,
            glob_map: Default::default(),

            used_imports: FxHashSet::default(),
            maybe_unused_trait_imports: Default::default(),
            maybe_unused_extern_crates: Vec::new(),

            unused_labels: FxHashMap::default(),

            privacy_errors: Vec::new(),
            ambiguity_errors: Vec::new(),
            use_injections: Vec::new(),
            macro_expanded_macro_export_errors: BTreeSet::new(),

            arenas,
            dummy_binding: arenas.alloc_name_binding(NameBinding {
                kind: NameBindingKind::Def(Def::Err, false),
                ambiguity: None,
                expansion: Mark::root(),
                span: DUMMY_SP,
                vis: ty::Visibility::Public,
            }),

            crate_loader,
            macro_names: FxHashSet::default(),
            builtin_macros: FxHashMap::default(),
            macro_use_prelude: FxHashMap::default(),
            all_macros: FxHashMap::default(),
            macro_map: FxHashMap::default(),
            invocations,
            macro_defs,
            local_macro_def_scopes: FxHashMap::default(),
            name_already_seen: FxHashMap::default(),
            potentially_unused_imports: Vec::new(),
            struct_constructors: Default::default(),
            unused_macros: FxHashSet::default(),
            current_type_ascription: Vec::new(),
            injected_crate: None,
        }
    }

    pub fn arenas() -> ResolverArenas<'a> {
        Default::default()
    }

    /// Runs the function on each namespace.
    fn per_ns<F: FnMut(&mut Self, Namespace)>(&mut self, mut f: F) {
        f(self, TypeNS);
        f(self, ValueNS);
        f(self, MacroNS);
    }

    fn macro_def(&self, mut ctxt: SyntaxContext) -> DefId {
        loop {
            match self.macro_defs.get(&ctxt.outer()) {
                Some(&def_id) => return def_id,
                None => ctxt.remove_mark(),
            };
        }
    }

    /// Entry point to crate resolution.
    pub fn resolve_crate(&mut self, krate: &Crate) {
        ImportResolver { resolver: self }.finalize_imports();
        self.current_module = self.graph_root;
        self.finalize_current_module_macro_resolutions();

        visit::walk_crate(self, krate);

        check_unused::check_crate(self, krate);
        self.report_errors(krate);
        self.crate_loader.postprocess(krate);
    }

    fn new_module(
        &self,
        parent: Module<'a>,
        kind: ModuleKind,
        normal_ancestor_id: DefId,
        expansion: Mark,
        span: Span,
    ) -> Module<'a> {
        let module = ModuleData::new(Some(parent), kind, normal_ancestor_id, expansion, span);
        self.arenas.alloc_module(module)
    }

    fn record_use(&mut self, ident: Ident, ns: Namespace,
                  used_binding: &'a NameBinding<'a>, is_lexical_scope: bool) {
        if let Some((b2, kind)) = used_binding.ambiguity {
            self.ambiguity_errors.push(AmbiguityError {
                kind, ident, b1: used_binding, b2,
                misc1: AmbiguityErrorMisc::None,
                misc2: AmbiguityErrorMisc::None,
            });
        }
        if let NameBindingKind::Import { directive, binding, ref used } = used_binding.kind {
            // Avoid marking `extern crate` items that refer to a name from extern prelude,
            // but not introduce it, as used if they are accessed from lexical scope.
            if is_lexical_scope {
                if let Some(entry) = self.extern_prelude.get(&ident.modern()) {
                    if let Some(crate_item) = entry.extern_crate_item {
                        if ptr::eq(used_binding, crate_item) && !entry.introduced_by_item {
                            return;
                        }
                    }
                }
            }
            used.set(true);
            directive.used.set(true);
            self.used_imports.insert((directive.id, ns));
            self.add_to_glob_map(directive.id, ident);
            self.record_use(ident, ns, binding, false);
        }
    }

    fn add_to_glob_map(&mut self, id: NodeId, ident: Ident) {
        if self.make_glob_map {
            self.glob_map.entry(id).or_default().insert(ident.name);
        }
    }

    /// This resolves the identifier `ident` in the namespace `ns` in the current lexical scope.
    /// More specifically, we proceed up the hierarchy of scopes and return the binding for
    /// `ident` in the first scope that defines it (or None if no scopes define it).
    ///
    /// A block's items are above its local variables in the scope hierarchy, regardless of where
    /// the items are defined in the block. For example,
    /// ```rust
    /// fn f() {
    ///    g(); // Since there are no local variables in scope yet, this resolves to the item.
    ///    let g = || {};
    ///    fn g() {}
    ///    g(); // This resolves to the local variable `g` since it shadows the item.
    /// }
    /// ```
    ///
    /// Invariant: This must only be called during main resolution, not during
    /// import resolution.
    fn resolve_ident_in_lexical_scope(&mut self,
                                      mut ident: Ident,
                                      ns: Namespace,
                                      record_used_id: Option<NodeId>,
                                      path_span: Span)
                                      -> Option<LexicalScopeBinding<'a>> {
        assert!(ns == TypeNS  || ns == ValueNS);
        if ident.name == keywords::Invalid.name() {
            return Some(LexicalScopeBinding::Def(Def::Err));
        }
        if ns == TypeNS {
            ident.span = if ident.name == keywords::SelfUpper.name() {
                // FIXME(jseyfried) improve `Self` hygiene
                ident.span.with_ctxt(SyntaxContext::empty())
            } else {
                ident.span.modern()
            }
        } else {
            ident = ident.modern_and_legacy();
        }

        // Walk backwards up the ribs in scope.
        let record_used = record_used_id.is_some();
        let mut module = self.graph_root;
        for i in (0 .. self.ribs[ns].len()).rev() {
            if let Some(def) = self.ribs[ns][i].bindings.get(&ident).cloned() {
                // The ident resolves to a type parameter or local variable.
                return Some(LexicalScopeBinding::Def(
                    self.adjust_local_def(ns, i, def, record_used, path_span)
                ));
            }

            module = match self.ribs[ns][i].kind {
                ModuleRibKind(module) => module,
                MacroDefinition(def) if def == self.macro_def(ident.span.ctxt()) => {
                    // If an invocation of this macro created `ident`, give up on `ident`
                    // and switch to `ident`'s source from the macro definition.
                    ident.span.remove_mark();
                    continue
                }
                _ => continue,
            };

            let item = self.resolve_ident_in_module_unadjusted(
                ModuleOrUniformRoot::Module(module),
                ident,
                ns,
                record_used,
                path_span,
            );
            if let Ok(binding) = item {
                // The ident resolves to an item.
                return Some(LexicalScopeBinding::Item(binding));
            }

            match module.kind {
                ModuleKind::Block(..) => {}, // We can see through blocks
                _ => break,
            }
        }

        ident.span = ident.span.modern();
        let mut poisoned = None;
        loop {
            let opt_module = if let Some(node_id) = record_used_id {
                self.hygienic_lexical_parent_with_compatibility_fallback(module, &mut ident.span,
                                                                         node_id, &mut poisoned)
            } else {
                self.hygienic_lexical_parent(module, &mut ident.span)
            };
            module = unwrap_or!(opt_module, break);
            let orig_current_module = self.current_module;
            self.current_module = module; // Lexical resolutions can never be a privacy error.
            let result = self.resolve_ident_in_module_unadjusted(
                ModuleOrUniformRoot::Module(module),
                ident,
                ns,
                record_used,
                path_span,
            );
            self.current_module = orig_current_module;

            match result {
                Ok(binding) => {
                    if let Some(node_id) = poisoned {
                        self.session.buffer_lint_with_diagnostic(
                            lint::builtin::PROC_MACRO_DERIVE_RESOLUTION_FALLBACK,
                            node_id, ident.span,
                            &format!("cannot find {} `{}` in this scope", ns.descr(), ident),
                            lint::builtin::BuiltinLintDiagnostics::
                                ProcMacroDeriveResolutionFallback(ident.span),
                        );
                    }
                    return Some(LexicalScopeBinding::Item(binding))
                }
                Err(Determined) => continue,
                Err(Undetermined) =>
                    span_bug!(ident.span, "undetermined resolution during main resolution pass"),
            }
        }

        if !module.no_implicit_prelude {
            if ns == TypeNS {
                if let Some(binding) = self.extern_prelude_get(ident, !record_used) {
                    return Some(LexicalScopeBinding::Item(binding));
                }
            }
            if ns == TypeNS && is_known_tool(ident.name) {
                let binding = (Def::ToolMod, ty::Visibility::Public,
                               DUMMY_SP, Mark::root()).to_name_binding(self.arenas);
                return Some(LexicalScopeBinding::Item(binding));
            }
            if let Some(prelude) = self.prelude {
                if let Ok(binding) = self.resolve_ident_in_module_unadjusted(
                    ModuleOrUniformRoot::Module(prelude),
                    ident,
                    ns,
                    false,
                    path_span,
                ) {
                    return Some(LexicalScopeBinding::Item(binding));
                }
            }
        }

        None
    }

    fn hygienic_lexical_parent(&mut self, module: Module<'a>, span: &mut Span)
                               -> Option<Module<'a>> {
        if !module.expansion.is_descendant_of(span.ctxt().outer()) {
            return Some(self.macro_def_scope(span.remove_mark()));
        }

        if let ModuleKind::Block(..) = module.kind {
            return Some(module.parent.unwrap());
        }

        None
    }

    fn hygienic_lexical_parent_with_compatibility_fallback(&mut self, module: Module<'a>,
                                                           span: &mut Span, node_id: NodeId,
                                                           poisoned: &mut Option<NodeId>)
                                                           -> Option<Module<'a>> {
        if let module @ Some(..) = self.hygienic_lexical_parent(module, span) {
            return module;
        }

        // We need to support the next case under a deprecation warning
        // ```
        // struct MyStruct;
        // ---- begin: this comes from a proc macro derive
        // mod implementation_details {
        //     // Note that `MyStruct` is not in scope here.
        //     impl SomeTrait for MyStruct { ... }
        // }
        // ---- end
        // ```
        // So we have to fall back to the module's parent during lexical resolution in this case.
        if let Some(parent) = module.parent {
            // Inner module is inside the macro, parent module is outside of the macro.
            if module.expansion != parent.expansion &&
            module.expansion.is_descendant_of(parent.expansion) {
                // The macro is a proc macro derive
                if module.expansion.looks_like_proc_macro_derive() {
                    if parent.expansion.is_descendant_of(span.ctxt().outer()) {
                        *poisoned = Some(node_id);
                        return module.parent;
                    }
                }
            }
        }

        None
    }

    fn resolve_ident_in_module(
        &mut self,
        module: ModuleOrUniformRoot<'a>,
        ident: Ident,
        ns: Namespace,
        parent_scope: Option<&ParentScope<'a>>,
        record_used: bool,
        path_span: Span
    ) -> Result<&'a NameBinding<'a>, Determinacy> {
        self.resolve_ident_in_module_ext(
            module, ident, ns, parent_scope, record_used, path_span
        ).map_err(|(determinacy, _)| determinacy)
    }

    fn resolve_ident_in_module_ext(
        &mut self,
        module: ModuleOrUniformRoot<'a>,
        mut ident: Ident,
        ns: Namespace,
        parent_scope: Option<&ParentScope<'a>>,
        record_used: bool,
        path_span: Span
    ) -> Result<&'a NameBinding<'a>, (Determinacy, Weak)> {
        let orig_current_module = self.current_module;
        match module {
            ModuleOrUniformRoot::Module(module) => {
                ident.span = ident.span.modern();
                if let Some(def) = ident.span.adjust(module.expansion) {
                    self.current_module = self.macro_def_scope(def);
                }
            }
            ModuleOrUniformRoot::ExternPrelude => {
                ident.span = ident.span.modern();
                ident.span.adjust(Mark::root());
            }
            ModuleOrUniformRoot::CrateRootAndExternPrelude |
            ModuleOrUniformRoot::CurrentScope => {
                // No adjustments
            }
        }
        let result = self.resolve_ident_in_module_unadjusted_ext(
            module, ident, ns, parent_scope, false, record_used, path_span,
        );
        self.current_module = orig_current_module;
        result
    }

    fn resolve_crate_root(&mut self, ident: Ident) -> Module<'a> {
        let mut ctxt = ident.span.ctxt();
        let mark = if ident.name == keywords::DollarCrate.name() {
            // When resolving `$crate` from a `macro_rules!` invoked in a `macro`,
            // we don't want to pretend that the `macro_rules!` definition is in the `macro`
            // as described in `SyntaxContext::apply_mark`, so we ignore prepended modern marks.
            // FIXME: This is only a guess and it doesn't work correctly for `macro_rules!`
            // definitions actually produced by `macro` and `macro` definitions produced by
            // `macro_rules!`, but at least such configurations are not stable yet.
            ctxt = ctxt.modern_and_legacy();
            let mut iter = ctxt.marks().into_iter().rev().peekable();
            let mut result = None;
            // Find the last modern mark from the end if it exists.
            while let Some(&(mark, transparency)) = iter.peek() {
                if transparency == Transparency::Opaque {
                    result = Some(mark);
                    iter.next();
                } else {
                    break;
                }
            }
            // Then find the last legacy mark from the end if it exists.
            for (mark, transparency) in iter {
                if transparency == Transparency::SemiTransparent {
                    result = Some(mark);
                } else {
                    break;
                }
            }
            result
        } else {
            ctxt = ctxt.modern();
            ctxt.adjust(Mark::root())
        };
        let module = match mark {
            Some(def) => self.macro_def_scope(def),
            None => return self.graph_root,
        };
        self.get_module(DefId { index: CRATE_DEF_INDEX, ..module.normal_ancestor_id })
    }

    fn resolve_self(&mut self, ctxt: &mut SyntaxContext, module: Module<'a>) -> Module<'a> {
        let mut module = self.get_module(module.normal_ancestor_id);
        while module.span.ctxt().modern() != *ctxt {
            let parent = module.parent.unwrap_or_else(|| self.macro_def_scope(ctxt.remove_mark()));
            module = self.get_module(parent.normal_ancestor_id);
        }
        module
    }

    // AST resolution
    //
    // We maintain a list of value ribs and type ribs.
    //
    // Simultaneously, we keep track of the current position in the module
    // graph in the `current_module` pointer. When we go to resolve a name in
    // the value or type namespaces, we first look through all the ribs and
    // then query the module graph. When we resolve a name in the module
    // namespace, we can skip all the ribs (since nested modules are not
    // allowed within blocks in Rust) and jump straight to the current module
    // graph node.
    //
    // Named implementations are handled separately. When we find a method
    // call, we consult the module node to find all of the implementations in
    // scope. This information is lazily cached in the module node. We then
    // generate a fake "implementation scope" containing all the
    // implementations thus found, for compatibility with old resolve pass.

    pub fn with_scope<F, T>(&mut self, id: NodeId, f: F) -> T
        where F: FnOnce(&mut Resolver) -> T
    {
        let id = self.definitions.local_def_id(id);
        let module = self.module_map.get(&id).cloned(); // clones a reference
        if let Some(module) = module {
            // Move down in the graph.
            let orig_module = replace(&mut self.current_module, module);
            self.ribs[ValueNS].push(Rib::new(ModuleRibKind(module)));
            self.ribs[TypeNS].push(Rib::new(ModuleRibKind(module)));

            self.finalize_current_module_macro_resolutions();
            let ret = f(self);

            self.current_module = orig_module;
            self.ribs[ValueNS].pop();
            self.ribs[TypeNS].pop();
            ret
        } else {
            f(self)
        }
    }

    /// Searches the current set of local scopes for labels. Returns the first non-None label that
    /// is returned by the given predicate function
    ///
    /// Stops after meeting a closure.
    fn search_label<P, R>(&self, mut ident: Ident, pred: P) -> Option<R>
        where P: Fn(&Rib, Ident) -> Option<R>
    {
        for rib in self.label_ribs.iter().rev() {
            match rib.kind {
                NormalRibKind => {}
                // If an invocation of this macro created `ident`, give up on `ident`
                // and switch to `ident`'s source from the macro definition.
                MacroDefinition(def) => {
                    if def == self.macro_def(ident.span.ctxt()) {
                        ident.span.remove_mark();
                    }
                }
                _ => {
                    // Do not resolve labels across function boundary
                    return None;
                }
            }
            let r = pred(rib, ident);
            if r.is_some() {
                return r;
            }
        }
        None
    }

    fn resolve_adt(&mut self, item: &Item, generics: &Generics) {
        self.with_current_self_item(item, |this| {
            this.with_type_parameter_rib(HasTypeParameters(generics, ItemRibKind), |this| {
                let item_def_id = this.definitions.local_def_id(item.id);
                this.with_self_rib(Def::SelfTy(None, Some(item_def_id)), |this| {
                    visit::walk_item(this, item);
                });
            });
        });
    }

    fn future_proof_import(&mut self, use_tree: &ast::UseTree) {
        let segments = &use_tree.prefix.segments;
        if !segments.is_empty() {
            let ident = segments[0].ident;
            if ident.is_path_segment_keyword() || ident.span.rust_2015() {
                return;
            }

            let nss = match use_tree.kind {
                ast::UseTreeKind::Simple(..) if segments.len() == 1 => &[TypeNS, ValueNS][..],
                _ => &[TypeNS],
            };
            for &ns in nss {
                if let Some(LexicalScopeBinding::Def(..)) =
                        self.resolve_ident_in_lexical_scope(ident, ns, None, use_tree.prefix.span) {
                    let what = if ns == TypeNS { "type parameters" } else { "local variables" };
                    self.session.span_err(ident.span, &format!("imports cannot refer to {}", what));
                }
            }
        } else if let ast::UseTreeKind::Nested(use_trees) = &use_tree.kind {
            for (use_tree, _) in use_trees {
                self.future_proof_import(use_tree);
            }
        }
    }

    fn resolve_item(&mut self, item: &Item) {
        let name = item.ident.name;
        debug!("(resolving item) resolving {}", name);

        match item.node {
            ItemKind::Ty(_, ref generics) |
            ItemKind::Fn(_, _, ref generics, _) |
            ItemKind::Existential(_, ref generics) => {
                self.with_type_parameter_rib(HasTypeParameters(generics, ItemRibKind),
                                             |this| visit::walk_item(this, item));
            }

            ItemKind::Enum(_, ref generics) |
            ItemKind::Struct(_, ref generics) |
            ItemKind::Union(_, ref generics) => {
                self.resolve_adt(item, generics);
            }

            ItemKind::Impl(.., ref generics, ref opt_trait_ref, ref self_type, ref impl_items) =>
                self.resolve_implementation(generics,
                                            opt_trait_ref,
                                            &self_type,
                                            item.id,
                                            impl_items),

            ItemKind::Trait(.., ref generics, ref bounds, ref trait_items) => {
                // Create a new rib for the trait-wide type parameters.
                self.with_type_parameter_rib(HasTypeParameters(generics, ItemRibKind), |this| {
                    let local_def_id = this.definitions.local_def_id(item.id);
                    this.with_self_rib(Def::SelfTy(Some(local_def_id), None), |this| {
                        this.visit_generics(generics);
                        walk_list!(this, visit_param_bound, bounds);

                        for trait_item in trait_items {
                            let type_parameters = HasTypeParameters(&trait_item.generics,
                                                                    TraitOrImplItemRibKind);
                            this.with_type_parameter_rib(type_parameters, |this| {
                                match trait_item.node {
                                    TraitItemKind::Const(ref ty, ref default) => {
                                        this.visit_ty(ty);

                                        // Only impose the restrictions of
                                        // ConstRibKind for an actual constant
                                        // expression in a provided default.
                                        if let Some(ref expr) = *default{
                                            this.with_constant_rib(|this| {
                                                this.visit_expr(expr);
                                            });
                                        }
                                    }
                                    TraitItemKind::Method(_, _) => {
                                        visit::walk_trait_item(this, trait_item)
                                    }
                                    TraitItemKind::Type(..) => {
                                        visit::walk_trait_item(this, trait_item)
                                    }
                                    TraitItemKind::Macro(_) => {
                                        panic!("unexpanded macro in resolve!")
                                    }
                                };
                            });
                        }
                    });
                });
            }

            ItemKind::TraitAlias(ref generics, ref bounds) => {
                // Create a new rib for the trait-wide type parameters.
                self.with_type_parameter_rib(HasTypeParameters(generics, ItemRibKind), |this| {
                    let local_def_id = this.definitions.local_def_id(item.id);
                    this.with_self_rib(Def::SelfTy(Some(local_def_id), None), |this| {
                        this.visit_generics(generics);
                        walk_list!(this, visit_param_bound, bounds);
                    });
                });
            }

            ItemKind::Mod(_) | ItemKind::ForeignMod(_) => {
                self.with_scope(item.id, |this| {
                    visit::walk_item(this, item);
                });
            }

            ItemKind::Static(ref ty, _, ref expr) |
            ItemKind::Const(ref ty, ref expr) => {
                self.with_item_rib(|this| {
                    this.visit_ty(ty);
                    this.with_constant_rib(|this| {
                        this.visit_expr(expr);
                    });
                });
            }

            ItemKind::Use(ref use_tree) => {
                self.future_proof_import(use_tree);
            }

            ItemKind::ExternCrate(..) |
            ItemKind::MacroDef(..) | ItemKind::GlobalAsm(..) => {
                // do nothing, these are just around to be encoded
            }

            ItemKind::Mac(_) => panic!("unexpanded macro in resolve!"),
        }
    }

    fn with_type_parameter_rib<'b, F>(&'b mut self, type_parameters: TypeParameters<'a, 'b>, f: F)
        where F: FnOnce(&mut Resolver)
    {
        match type_parameters {
            HasTypeParameters(generics, rib_kind) => {
                let mut function_type_rib = Rib::new(rib_kind);
                let mut seen_bindings = FxHashMap::default();
                for param in &generics.params {
                    match param.kind {
                        GenericParamKind::Lifetime { .. } => {}
                        GenericParamKind::Type { .. } => {
                            let ident = param.ident.modern();
                            debug!("with_type_parameter_rib: {}", param.id);

                            if seen_bindings.contains_key(&ident) {
                                let span = seen_bindings.get(&ident).unwrap();
                                let err = ResolutionError::NameAlreadyUsedInTypeParameterList(
                                    ident.name,
                                    span,
                                );
                                resolve_error(self, param.ident.span, err);
                            }
                            seen_bindings.entry(ident).or_insert(param.ident.span);

                        // Plain insert (no renaming).
                        let def = Def::TyParam(self.definitions.local_def_id(param.id));
                            function_type_rib.bindings.insert(ident, def);
                            self.record_def(param.id, PathResolution::new(def));
                        }
                    }
                }
                self.ribs[TypeNS].push(function_type_rib);
            }

            NoTypeParameters => {
                // Nothing to do.
            }
        }

        f(self);

        if let HasTypeParameters(..) = type_parameters {
            self.ribs[TypeNS].pop();
        }
    }

    fn with_label_rib<F>(&mut self, f: F)
        where F: FnOnce(&mut Resolver)
    {
        self.label_ribs.push(Rib::new(NormalRibKind));
        f(self);
        self.label_ribs.pop();
    }

    fn with_item_rib<F>(&mut self, f: F)
        where F: FnOnce(&mut Resolver)
    {
        self.ribs[ValueNS].push(Rib::new(ItemRibKind));
        self.ribs[TypeNS].push(Rib::new(ItemRibKind));
        f(self);
        self.ribs[TypeNS].pop();
        self.ribs[ValueNS].pop();
    }

    fn with_constant_rib<F>(&mut self, f: F)
        where F: FnOnce(&mut Resolver)
    {
        self.ribs[ValueNS].push(Rib::new(ConstantItemRibKind));
        self.label_ribs.push(Rib::new(ConstantItemRibKind));
        f(self);
        self.label_ribs.pop();
        self.ribs[ValueNS].pop();
    }

    fn with_current_self_type<T, F>(&mut self, self_type: &Ty, f: F) -> T
        where F: FnOnce(&mut Resolver) -> T
    {
        // Handle nested impls (inside fn bodies)
        let previous_value = replace(&mut self.current_self_type, Some(self_type.clone()));
        let result = f(self);
        self.current_self_type = previous_value;
        result
    }

    fn with_current_self_item<T, F>(&mut self, self_item: &Item, f: F) -> T
        where F: FnOnce(&mut Resolver) -> T
    {
        let previous_value = replace(&mut self.current_self_item, Some(self_item.id));
        let result = f(self);
        self.current_self_item = previous_value;
        result
    }

    /// This is called to resolve a trait reference from an `impl` (i.e., `impl Trait for Foo`)
    fn with_optional_trait_ref<T, F>(&mut self, opt_trait_ref: Option<&TraitRef>, f: F) -> T
        where F: FnOnce(&mut Resolver, Option<DefId>) -> T
    {
        let mut new_val = None;
        let mut new_id = None;
        if let Some(trait_ref) = opt_trait_ref {
            let path: Vec<_> = Segment::from_path(&trait_ref.path);
            let def = self.smart_resolve_path_fragment(
                trait_ref.ref_id,
                None,
                &path,
                trait_ref.path.span,
                PathSource::Trait(AliasPossibility::No),
                CrateLint::SimplePath(trait_ref.ref_id),
            ).base_def();
            if def != Def::Err {
                new_id = Some(def.def_id());
                let span = trait_ref.path.span;
                if let PathResult::Module(ModuleOrUniformRoot::Module(module)) =
                    self.resolve_path_without_parent_scope(
                        &path,
                        Some(TypeNS),
                        false,
                        span,
                        CrateLint::SimplePath(trait_ref.ref_id),
                    )
                {
                    new_val = Some((module, trait_ref.clone()));
                }
            }
        }
        let original_trait_ref = replace(&mut self.current_trait_ref, new_val);
        let result = f(self, new_id);
        self.current_trait_ref = original_trait_ref;
        result
    }

    fn with_self_rib<F>(&mut self, self_def: Def, f: F)
        where F: FnOnce(&mut Resolver)
    {
        let mut self_type_rib = Rib::new(NormalRibKind);

        // plain insert (no renaming, types are not currently hygienic....)
        self_type_rib.bindings.insert(keywords::SelfUpper.ident(), self_def);
        self.ribs[TypeNS].push(self_type_rib);
        f(self);
        self.ribs[TypeNS].pop();
    }

    fn with_self_struct_ctor_rib<F>(&mut self, impl_id: DefId, f: F)
        where F: FnOnce(&mut Resolver)
    {
        let self_def = Def::SelfCtor(impl_id);
        let mut self_type_rib = Rib::new(NormalRibKind);
        self_type_rib.bindings.insert(keywords::SelfUpper.ident(), self_def);
        self.ribs[ValueNS].push(self_type_rib);
        f(self);
        self.ribs[ValueNS].pop();
    }

    fn resolve_implementation(&mut self,
                              generics: &Generics,
                              opt_trait_reference: &Option<TraitRef>,
                              self_type: &Ty,
                              item_id: NodeId,
                              impl_items: &[ImplItem]) {
        // If applicable, create a rib for the type parameters.
        self.with_type_parameter_rib(HasTypeParameters(generics, ItemRibKind), |this| {
            // Dummy self type for better errors if `Self` is used in the trait path.
            this.with_self_rib(Def::SelfTy(None, None), |this| {
                // Resolve the trait reference, if necessary.
                this.with_optional_trait_ref(opt_trait_reference.as_ref(), |this, trait_id| {
                    let item_def_id = this.definitions.local_def_id(item_id);
                    this.with_self_rib(Def::SelfTy(trait_id, Some(item_def_id)), |this| {
                        if let Some(trait_ref) = opt_trait_reference.as_ref() {
                            // Resolve type arguments in the trait path.
                            visit::walk_trait_ref(this, trait_ref);
                        }
                        // Resolve the self type.
                        this.visit_ty(self_type);
                        // Resolve the type parameters.
                        this.visit_generics(generics);
                        // Resolve the items within the impl.
                        this.with_current_self_type(self_type, |this| {
                            this.with_self_struct_ctor_rib(item_def_id, |this| {
                                for impl_item in impl_items {
                                    this.resolve_visibility(&impl_item.vis);

                                    // We also need a new scope for the impl item type parameters.
                                    let type_parameters = HasTypeParameters(&impl_item.generics,
                                                                            TraitOrImplItemRibKind);
                                    this.with_type_parameter_rib(type_parameters, |this| {
                                        use self::ResolutionError::*;
                                        match impl_item.node {
                                            ImplItemKind::Const(..) => {
                                                // If this is a trait impl, ensure the const
                                                // exists in trait
                                                this.check_trait_item(impl_item.ident,
                                                                      ValueNS,
                                                                      impl_item.span,
                                                    |n, s| ConstNotMemberOfTrait(n, s));
                                                this.with_constant_rib(|this|
                                                    visit::walk_impl_item(this, impl_item)
                                                );
                                            }
                                            ImplItemKind::Method(..) => {
                                                // If this is a trait impl, ensure the method
                                                // exists in trait
                                                this.check_trait_item(impl_item.ident,
                                                                      ValueNS,
                                                                      impl_item.span,
                                                    |n, s| MethodNotMemberOfTrait(n, s));

                                                visit::walk_impl_item(this, impl_item);
                                            }
                                            ImplItemKind::Type(ref ty) => {
                                                // If this is a trait impl, ensure the type
                                                // exists in trait
                                                this.check_trait_item(impl_item.ident,
                                                                      TypeNS,
                                                                      impl_item.span,
                                                    |n, s| TypeNotMemberOfTrait(n, s));

                                                this.visit_ty(ty);
                                            }
                                            ImplItemKind::Existential(ref bounds) => {
                                                // If this is a trait impl, ensure the type
                                                // exists in trait
                                                this.check_trait_item(impl_item.ident,
                                                                      TypeNS,
                                                                      impl_item.span,
                                                    |n, s| TypeNotMemberOfTrait(n, s));

                                                for bound in bounds {
                                                    this.visit_param_bound(bound);
                                                }
                                            }
                                            ImplItemKind::Macro(_) =>
                                                panic!("unexpanded macro in resolve!"),
                                        }
                                    });
                                }
                            });
                        });
                    });
                });
            });
        });
    }

    fn check_trait_item<F>(&mut self, ident: Ident, ns: Namespace, span: Span, err: F)
        where F: FnOnce(Name, &str) -> ResolutionError
    {
        // If there is a TraitRef in scope for an impl, then the method must be in the
        // trait.
        if let Some((module, _)) = self.current_trait_ref {
            if self.resolve_ident_in_module(
                ModuleOrUniformRoot::Module(module),
                ident,
                ns,
                None,
                false,
                span,
            ).is_err() {
                let path = &self.current_trait_ref.as_ref().unwrap().1.path;
                resolve_error(self, span, err(ident.name, &path_names_to_string(path)));
            }
        }
    }

    fn resolve_local(&mut self, local: &Local) {
        // Resolve the type.
        walk_list!(self, visit_ty, &local.ty);

        // Resolve the initializer.
        walk_list!(self, visit_expr, &local.init);

        // Resolve the pattern.
        self.resolve_pattern(&local.pat, PatternSource::Let, &mut FxHashMap::default());
    }

    // build a map from pattern identifiers to binding-info's.
    // this is done hygienically. This could arise for a macro
    // that expands into an or-pattern where one 'x' was from the
    // user and one 'x' came from the macro.
    fn binding_mode_map(&mut self, pat: &Pat) -> BindingMap {
        let mut binding_map = FxHashMap::default();

        pat.walk(&mut |pat| {
            if let PatKind::Ident(binding_mode, ident, ref sub_pat) = pat.node {
                if sub_pat.is_some() || match self.def_map.get(&pat.id).map(|res| res.base_def()) {
                    Some(Def::Local(..)) => true,
                    _ => false,
                } {
                    let binding_info = BindingInfo { span: ident.span, binding_mode: binding_mode };
                    binding_map.insert(ident, binding_info);
                }
            }
            true
        });

        binding_map
    }

    // check that all of the arms in an or-pattern have exactly the
    // same set of bindings, with the same binding modes for each.
    fn check_consistent_bindings(&mut self, pats: &[P<Pat>]) {
        if pats.is_empty() {
            return;
        }

        let mut missing_vars = FxHashMap::default();
        let mut inconsistent_vars = FxHashMap::default();
        for (i, p) in pats.iter().enumerate() {
            let map_i = self.binding_mode_map(&p);

            for (j, q) in pats.iter().enumerate() {
                if i == j {
                    continue;
                }

                let map_j = self.binding_mode_map(&q);
                for (&key, &binding_i) in &map_i {
                    if map_j.is_empty() {                   // Account for missing bindings when
                        let binding_error = missing_vars    // map_j has none.
                            .entry(key.name)
                            .or_insert(BindingError {
                                name: key.name,
                                origin: BTreeSet::new(),
                                target: BTreeSet::new(),
                            });
                        binding_error.origin.insert(binding_i.span);
                        binding_error.target.insert(q.span);
                    }
                    for (&key_j, &binding_j) in &map_j {
                        match map_i.get(&key_j) {
                            None => {  // missing binding
                                let binding_error = missing_vars
                                    .entry(key_j.name)
                                    .or_insert(BindingError {
                                        name: key_j.name,
                                        origin: BTreeSet::new(),
                                        target: BTreeSet::new(),
                                    });
                                binding_error.origin.insert(binding_j.span);
                                binding_error.target.insert(p.span);
                            }
                            Some(binding_i) => {  // check consistent binding
                                if binding_i.binding_mode != binding_j.binding_mode {
                                    inconsistent_vars
                                        .entry(key.name)
                                        .or_insert((binding_j.span, binding_i.span));
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut missing_vars = missing_vars.iter().collect::<Vec<_>>();
        missing_vars.sort();
        for (_, v) in missing_vars {
            resolve_error(self,
                          *v.origin.iter().next().unwrap(),
                          ResolutionError::VariableNotBoundInPattern(v));
        }
        let mut inconsistent_vars = inconsistent_vars.iter().collect::<Vec<_>>();
        inconsistent_vars.sort();
        for (name, v) in inconsistent_vars {
            resolve_error(self, v.0, ResolutionError::VariableBoundWithDifferentMode(*name, v.1));
        }
    }

    fn resolve_arm(&mut self, arm: &Arm) {
        self.ribs[ValueNS].push(Rib::new(NormalRibKind));

        let mut bindings_list = FxHashMap::default();
        for pattern in &arm.pats {
            self.resolve_pattern(&pattern, PatternSource::Match, &mut bindings_list);
        }

        // This has to happen *after* we determine which pat_idents are variants.
        self.check_consistent_bindings(&arm.pats);

        if let Some(ast::Guard::If(ref expr)) = arm.guard {
            self.visit_expr(expr)
        }
        self.visit_expr(&arm.body);

        self.ribs[ValueNS].pop();
    }

    fn resolve_block(&mut self, block: &Block) {
        debug!("(resolving block) entering block");
        // Move down in the graph, if there's an anonymous module rooted here.
        let orig_module = self.current_module;
        let anonymous_module = self.block_map.get(&block.id).cloned(); // clones a reference

        let mut num_macro_definition_ribs = 0;
        if let Some(anonymous_module) = anonymous_module {
            debug!("(resolving block) found anonymous module, moving down");
            self.ribs[ValueNS].push(Rib::new(ModuleRibKind(anonymous_module)));
            self.ribs[TypeNS].push(Rib::new(ModuleRibKind(anonymous_module)));
            self.current_module = anonymous_module;
            self.finalize_current_module_macro_resolutions();
        } else {
            self.ribs[ValueNS].push(Rib::new(NormalRibKind));
        }

        // Descend into the block.
        for stmt in &block.stmts {
            if let ast::StmtKind::Item(ref item) = stmt.node {
                if let ast::ItemKind::MacroDef(..) = item.node {
                    num_macro_definition_ribs += 1;
                    let def = self.definitions.local_def_id(item.id);
                    self.ribs[ValueNS].push(Rib::new(MacroDefinition(def)));
                    self.label_ribs.push(Rib::new(MacroDefinition(def)));
                }
            }

            self.visit_stmt(stmt);
        }

        // Move back up.
        self.current_module = orig_module;
        for _ in 0 .. num_macro_definition_ribs {
            self.ribs[ValueNS].pop();
            self.label_ribs.pop();
        }
        self.ribs[ValueNS].pop();
        if anonymous_module.is_some() {
            self.ribs[TypeNS].pop();
        }
        debug!("(resolving block) leaving block");
    }

    fn fresh_binding(&mut self,
                     ident: Ident,
                     pat_id: NodeId,
                     outer_pat_id: NodeId,
                     pat_src: PatternSource,
                     bindings: &mut FxHashMap<Ident, NodeId>)
                     -> PathResolution {
        // Add the binding to the local ribs, if it
        // doesn't already exist in the bindings map. (We
        // must not add it if it's in the bindings map
        // because that breaks the assumptions later
        // passes make about or-patterns.)
        let ident = ident.modern_and_legacy();
        let mut def = Def::Local(pat_id);
        match bindings.get(&ident).cloned() {
            Some(id) if id == outer_pat_id => {
                // `Variant(a, a)`, error
                resolve_error(
                    self,
                    ident.span,
                    ResolutionError::IdentifierBoundMoreThanOnceInSamePattern(
                        &ident.as_str())
                );
            }
            Some(..) if pat_src == PatternSource::FnParam => {
                // `fn f(a: u8, a: u8)`, error
                resolve_error(
                    self,
                    ident.span,
                    ResolutionError::IdentifierBoundMoreThanOnceInParameterList(
                        &ident.as_str())
                );
            }
            Some(..) if pat_src == PatternSource::Match ||
                        pat_src == PatternSource::IfLet ||
                        pat_src == PatternSource::WhileLet => {
                // `Variant1(a) | Variant2(a)`, ok
                // Reuse definition from the first `a`.
                def = self.ribs[ValueNS].last_mut().unwrap().bindings[&ident];
            }
            Some(..) => {
                span_bug!(ident.span, "two bindings with the same name from \
                                       unexpected pattern source {:?}", pat_src);
            }
            None => {
                // A completely fresh binding, add to the lists if it's valid.
                if ident.name != keywords::Invalid.name() {
                    bindings.insert(ident, outer_pat_id);
                    self.ribs[ValueNS].last_mut().unwrap().bindings.insert(ident, def);
                }
            }
        }

        PathResolution::new(def)
    }

    fn resolve_pattern(&mut self,
                       pat: &Pat,
                       pat_src: PatternSource,
                       // Maps idents to the node ID for the
                       // outermost pattern that binds them.
                       bindings: &mut FxHashMap<Ident, NodeId>) {
        // Visit all direct subpatterns of this pattern.
        let outer_pat_id = pat.id;
        pat.walk(&mut |pat| {
            debug!("resolve_pattern pat={:?} node={:?}", pat, pat.node);
            match pat.node {
                PatKind::Ident(bmode, ident, ref opt_pat) => {
                    // First try to resolve the identifier as some existing
                    // entity, then fall back to a fresh binding.
                    let binding = self.resolve_ident_in_lexical_scope(ident, ValueNS,
                                                                      None, pat.span)
                                      .and_then(LexicalScopeBinding::item);
                    let resolution = binding.map(NameBinding::def).and_then(|def| {
                        let is_syntactic_ambiguity = opt_pat.is_none() &&
                            bmode == BindingMode::ByValue(Mutability::Immutable);
                        match def {
                            Def::StructCtor(_, CtorKind::Const) |
                            Def::VariantCtor(_, CtorKind::Const) |
                            Def::Const(..) if is_syntactic_ambiguity => {
                                // Disambiguate in favor of a unit struct/variant
                                // or constant pattern.
                                self.record_use(ident, ValueNS, binding.unwrap(), false);
                                Some(PathResolution::new(def))
                            }
                            Def::StructCtor(..) | Def::VariantCtor(..) |
                            Def::Const(..) | Def::Static(..) => {
                                // This is unambiguously a fresh binding, either syntactically
                                // (e.g., `IDENT @ PAT` or `ref IDENT`) or because `IDENT` resolves
                                // to something unusable as a pattern (e.g., constructor function),
                                // but we still conservatively report an error, see
                                // issues/33118#issuecomment-233962221 for one reason why.
                                resolve_error(
                                    self,
                                    ident.span,
                                    ResolutionError::BindingShadowsSomethingUnacceptable(
                                        pat_src.descr(), ident.name, binding.unwrap())
                                );
                                None
                            }
                            Def::Fn(..) | Def::Err => {
                                // These entities are explicitly allowed
                                // to be shadowed by fresh bindings.
                                None
                            }
                            def => {
                                span_bug!(ident.span, "unexpected definition for an \
                                                       identifier in pattern: {:?}", def);
                            }
                        }
                    }).unwrap_or_else(|| {
                        self.fresh_binding(ident, pat.id, outer_pat_id, pat_src, bindings)
                    });

                    self.record_def(pat.id, resolution);
                }

                PatKind::TupleStruct(ref path, ..) => {
                    self.smart_resolve_path(pat.id, None, path, PathSource::TupleStruct);
                }

                PatKind::Path(ref qself, ref path) => {
                    self.smart_resolve_path(pat.id, qself.as_ref(), path, PathSource::Pat);
                }

                PatKind::Struct(ref path, ..) => {
                    self.smart_resolve_path(pat.id, None, path, PathSource::Struct);
                }

                _ => {}
            }
            true
        });

        visit::walk_pat(self, pat);
    }

    // High-level and context dependent path resolution routine.
    // Resolves the path and records the resolution into definition map.
    // If resolution fails tries several techniques to find likely
    // resolution candidates, suggest imports or other help, and report
    // errors in user friendly way.
    fn smart_resolve_path(&mut self,
                          id: NodeId,
                          qself: Option<&QSelf>,
                          path: &Path,
                          source: PathSource)
                          -> PathResolution {
        self.smart_resolve_path_with_crate_lint(id, qself, path, source, CrateLint::SimplePath(id))
    }

    /// A variant of `smart_resolve_path` where you also specify extra
    /// information about where the path came from; this extra info is
    /// sometimes needed for the lint that recommends rewriting
    /// absolute paths to `crate`, so that it knows how to frame the
    /// suggestion. If you are just resolving a path like `foo::bar`
    /// that appears...somewhere, though, then you just want
    /// `CrateLint::SimplePath`, which is what `smart_resolve_path`
    /// already provides.
    fn smart_resolve_path_with_crate_lint(
        &mut self,
        id: NodeId,
        qself: Option<&QSelf>,
        path: &Path,
        source: PathSource,
        crate_lint: CrateLint
    ) -> PathResolution {
        self.smart_resolve_path_fragment(
            id,
            qself,
            &Segment::from_path(path),
            path.span,
            source,
            crate_lint,
        )
    }

    fn smart_resolve_path_fragment(&mut self,
                                   id: NodeId,
                                   qself: Option<&QSelf>,
                                   path: &[Segment],
                                   span: Span,
                                   source: PathSource,
                                   crate_lint: CrateLint)
                                   -> PathResolution {
        let ident_span = path.last().map_or(span, |ident| ident.ident.span);
        let ns = source.namespace();
        let is_expected = &|def| source.is_expected(def);
        let is_enum_variant = &|def| if let Def::Variant(..) = def { true } else { false };

        // Base error is amended with one short label and possibly some longer helps/notes.
        let report_errors = |this: &mut Self, def: Option<Def>| {
            // Make the base error.
            let expected = source.descr_expected();
            let path_str = Segment::names_to_string(path);
            let item_str = path.last().unwrap().ident;
            let code = source.error_code(def.is_some());
            let (base_msg, fallback_label, base_span) = if let Some(def) = def {
                (format!("expected {}, found {} `{}`", expected, def.kind_name(), path_str),
                 format!("not a {}", expected),
                 span)
            } else {
                let item_span = path.last().unwrap().ident.span;
                let (mod_prefix, mod_str) = if path.len() == 1 {
                    (String::new(), "this scope".to_string())
                } else if path.len() == 2 && path[0].ident.name == keywords::PathRoot.name() {
                    (String::new(), "the crate root".to_string())
                } else {
                    let mod_path = &path[..path.len() - 1];
                    let mod_prefix = match this.resolve_path_without_parent_scope(
                        mod_path, Some(TypeNS), false, span, CrateLint::No
                    ) {
                        PathResult::Module(ModuleOrUniformRoot::Module(module)) =>
                            module.def(),
                        _ => None,
                    }.map_or(String::new(), |def| format!("{} ", def.kind_name()));
                    (mod_prefix, format!("`{}`", Segment::names_to_string(mod_path)))
                };
                (format!("cannot find {} `{}` in {}{}", expected, item_str, mod_prefix, mod_str),
                 format!("not found in {}", mod_str),
                 item_span)
            };

            let code = DiagnosticId::Error(code.into());
            let mut err = this.session.struct_span_err_with_code(base_span, &base_msg, code);

            // Emit help message for fake-self from other languages like `this`(javascript)
            if ["this", "my"].contains(&&*item_str.as_str())
                && this.self_value_is_available(path[0].ident.span, span) {
                err.span_suggestion_with_applicability(
                    span,
                    "did you mean",
                    "self".to_string(),
                    Applicability::MaybeIncorrect,
                );
            }

            // Emit special messages for unresolved `Self` and `self`.
            if is_self_type(path, ns) {
                __diagnostic_used!(E0411);
                err.code(DiagnosticId::Error("E0411".into()));
                err.span_label(span, format!("`Self` is only available in impls, traits, \
                                              and type definitions"));
                return (err, Vec::new());
            }
            if is_self_value(path, ns) {
                debug!("smart_resolve_path_fragment E0424 source:{:?}", source);

                __diagnostic_used!(E0424);
                err.code(DiagnosticId::Error("E0424".into()));
                err.span_label(span, match source {
                    PathSource::Pat => {
                        format!("`self` value is a keyword \
                                and may not be bound to \
                                variables or shadowed")
                    }
                    _ => {
                        format!("`self` value is a keyword \
                                only available in methods \
                                with `self` parameter")
                    }
                });
                return (err, Vec::new());
            }

            // Try to lookup the name in more relaxed fashion for better error reporting.
            let ident = path.last().unwrap().ident;
            let candidates = this.lookup_import_candidates(ident, ns, is_expected);
            if candidates.is_empty() && is_expected(Def::Enum(DefId::local(CRATE_DEF_INDEX))) {
                let enum_candidates =
                    this.lookup_import_candidates(ident, ns, is_enum_variant);
                let mut enum_candidates = enum_candidates.iter()
                    .map(|suggestion| {
                        import_candidate_to_enum_paths(&suggestion)
                    }).collect::<Vec<_>>();
                enum_candidates.sort();

                if !enum_candidates.is_empty() {
                    // contextualize for E0412 "cannot find type", but don't belabor the point
                    // (that it's a variant) for E0573 "expected type, found variant"
                    let preamble = if def.is_none() {
                        let others = match enum_candidates.len() {
                            1 => String::new(),
                            2 => " and 1 other".to_owned(),
                            n => format!(" and {} others", n)
                        };
                        format!("there is an enum variant `{}`{}; ",
                                enum_candidates[0].0, others)
                    } else {
                        String::new()
                    };
                    let msg = format!("{}try using the variant's enum", preamble);

                    err.span_suggestions_with_applicability(
                        span,
                        &msg,
                        enum_candidates.into_iter()
                            .map(|(_variant_path, enum_ty_path)| enum_ty_path)
                            // variants reëxported in prelude doesn't mean `prelude::v1` is the
                            // type name! FIXME: is there a more principled way to do this that
                            // would work for other reëxports?
                            .filter(|enum_ty_path| enum_ty_path != "std::prelude::v1")
                            // also say `Option` rather than `std::prelude::v1::Option`
                            .map(|enum_ty_path| {
                                // FIXME #56861: DRYer prelude filtering
                                enum_ty_path.trim_start_matches("std::prelude::v1::").to_owned()
                            }),
                        Applicability::MachineApplicable,
                    );
                }
            }
            if path.len() == 1 && this.self_type_is_available(span) {
                if let Some(candidate) = this.lookup_assoc_candidate(ident, ns, is_expected) {
                    let self_is_available = this.self_value_is_available(path[0].ident.span, span);
                    match candidate {
                        AssocSuggestion::Field => {
                            err.span_suggestion_with_applicability(
                                span,
                                "try",
                                format!("self.{}", path_str),
                                Applicability::MachineApplicable,
                            );
                            if !self_is_available {
                                err.span_label(span, format!("`self` value is a keyword \
                                                               only available in \
                                                               methods with `self` parameter"));
                            }
                        }
                        AssocSuggestion::MethodWithSelf if self_is_available => {
                            err.span_suggestion_with_applicability(
                                span,
                                "try",
                                format!("self.{}", path_str),
                                Applicability::MachineApplicable,
                            );
                        }
                        AssocSuggestion::MethodWithSelf | AssocSuggestion::AssocItem => {
                            err.span_suggestion_with_applicability(
                                span,
                                "try",
                                format!("Self::{}", path_str),
                                Applicability::MachineApplicable,
                            );
                        }
                    }
                    return (err, candidates);
                }
            }

            let mut levenshtein_worked = false;

            // Try Levenshtein algorithm.
            if let Some(candidate) = this.lookup_typo_candidate(path, ns, is_expected, span) {
                err.span_label(ident_span, format!("did you mean `{}`?", candidate));
                levenshtein_worked = true;
            }

            // Try context dependent help if relaxed lookup didn't work.
            if let Some(def) = def {
                match (def, source) {
                    (Def::Macro(..), _) => {
                        err.span_label(span, format!("did you mean `{}!(...)`?", path_str));
                        return (err, candidates);
                    }
                    (Def::TyAlias(..), PathSource::Trait(_)) => {
                        err.span_label(span, "type aliases cannot be used as traits");
                        if nightly_options::is_nightly_build() {
                            err.note("did you mean to use a trait alias?");
                        }
                        return (err, candidates);
                    }
                    (Def::Mod(..), PathSource::Expr(Some(parent))) => match parent.node {
                        ExprKind::Field(_, ident) => {
                            err.span_label(parent.span, format!("did you mean `{}::{}`?",
                                                                 path_str, ident));
                            return (err, candidates);
                        }
                        ExprKind::MethodCall(ref segment, ..) => {
                            err.span_label(parent.span, format!("did you mean `{}::{}(...)`?",
                                                                 path_str, segment.ident));
                            return (err, candidates);
                        }
                        _ => {}
                    },
                    (Def::Enum(..), PathSource::TupleStruct)
                        | (Def::Enum(..), PathSource::Expr(..))  => {
                        if let Some(variants) = this.collect_enum_variants(def) {
                            err.note(&format!("did you mean to use one \
                                               of the following variants?\n{}",
                                variants.iter()
                                    .map(|suggestion| path_names_to_string(suggestion))
                                    .map(|suggestion| format!("- `{}`", suggestion))
                                    .collect::<Vec<_>>()
                                    .join("\n")));

                        } else {
                            err.note("did you mean to use one of the enum's variants?");
                        }
                        return (err, candidates);
                    },
                    (Def::Struct(def_id), _) if ns == ValueNS => {
                        if let Some((ctor_def, ctor_vis))
                                = this.struct_constructors.get(&def_id).cloned() {
                            let accessible_ctor = this.is_accessible(ctor_vis);
                            if is_expected(ctor_def) && !accessible_ctor {
                                err.span_label(span, format!("constructor is not visible \
                                                              here due to private fields"));
                            }
                        } else {
                            // HACK(estebank): find a better way to figure out that this was a
                            // parser issue where a struct literal is being used on an expression
                            // where a brace being opened means a block is being started. Look
                            // ahead for the next text to see if `span` is followed by a `{`.
                            let sm = this.session.source_map();
                            let mut sp = span;
                            loop {
                                sp = sm.next_point(sp);
                                match sm.span_to_snippet(sp) {
                                    Ok(ref snippet) => {
                                        if snippet.chars().any(|c| { !c.is_whitespace() }) {
                                            break;
                                        }
                                    }
                                    _ => break,
                                }
                            }
                            let followed_by_brace = match sm.span_to_snippet(sp) {
                                Ok(ref snippet) if snippet == "{" => true,
                                _ => false,
                            };
                            match source {
                                PathSource::Expr(Some(parent)) => {
                                    match parent.node {
                                        ExprKind::MethodCall(ref path_assignment, _)  => {
                                            err.span_suggestion_with_applicability(
                                                sm.start_point(parent.span)
                                                  .to(path_assignment.ident.span),
                                                "use `::` to access an associated function",
                                                format!("{}::{}",
                                                        path_str,
                                                        path_assignment.ident),
                                                Applicability::MaybeIncorrect
                                            );
                                            return (err, candidates);
                                        },
                                        _ => {
                                            err.span_label(
                                                span,
                                                format!("did you mean `{} {{ /* fields */ }}`?",
                                                        path_str),
                                            );
                                            return (err, candidates);
                                        },
                                    }
                                },
                                PathSource::Expr(None) if followed_by_brace == true => {
                                    err.span_label(
                                        span,
                                        format!("did you mean `({} {{ /* fields */ }})`?",
                                                path_str),
                                    );
                                    return (err, candidates);
                                },
                                _ => {
                                    err.span_label(
                                        span,
                                        format!("did you mean `{} {{ /* fields */ }}`?",
                                                path_str),
                                    );
                                    return (err, candidates);
                                },
                            }
                        }
                        return (err, candidates);
                    }
                    (Def::Union(..), _) |
                    (Def::Variant(..), _) |
                    (Def::VariantCtor(_, CtorKind::Fictive), _) if ns == ValueNS => {
                        err.span_label(span, format!("did you mean `{} {{ /* fields */ }}`?",
                                                     path_str));
                        return (err, candidates);
                    }
                    (Def::SelfTy(..), _) if ns == ValueNS => {
                        err.span_label(span, fallback_label);
                        err.note("can't use `Self` as a constructor, you must use the \
                                  implemented struct");
                        return (err, candidates);
                    }
                    (Def::TyAlias(_), _) | (Def::AssociatedTy(..), _) if ns == ValueNS => {
                        err.note("can't use a type alias as a constructor");
                        return (err, candidates);
                    }
                    _ => {}
                }
            }

            // Fallback label.
            if !levenshtein_worked {
                err.span_label(base_span, fallback_label);
                this.type_ascription_suggestion(&mut err, base_span);
            }
            (err, candidates)
        };
        let report_errors = |this: &mut Self, def: Option<Def>| {
            let (err, candidates) = report_errors(this, def);
            let def_id = this.current_module.normal_ancestor_id;
            let node_id = this.definitions.as_local_node_id(def_id).unwrap();
            let better = def.is_some();
            this.use_injections.push(UseError { err, candidates, node_id, better });
            err_path_resolution()
        };

        let resolution = match self.resolve_qpath_anywhere(
            id,
            qself,
            path,
            ns,
            span,
            source.defer_to_typeck(),
            source.global_by_default(),
            crate_lint,
        ) {
            Some(resolution) if resolution.unresolved_segments() == 0 => {
                if is_expected(resolution.base_def()) || resolution.base_def() == Def::Err {
                    resolution
                } else {
                    // Add a temporary hack to smooth the transition to new struct ctor
                    // visibility rules. See #38932 for more details.
                    let mut res = None;
                    if let Def::Struct(def_id) = resolution.base_def() {
                        if let Some((ctor_def, ctor_vis))
                                = self.struct_constructors.get(&def_id).cloned() {
                            if is_expected(ctor_def) && self.is_accessible(ctor_vis) {
                                let lint = lint::builtin::LEGACY_CONSTRUCTOR_VISIBILITY;
                                self.session.buffer_lint(lint, id, span,
                                    "private struct constructors are not usable through \
                                     re-exports in outer modules",
                                );
                                res = Some(PathResolution::new(ctor_def));
                            }
                        }
                    }

                    res.unwrap_or_else(|| report_errors(self, Some(resolution.base_def())))
                }
            }
            Some(resolution) if source.defer_to_typeck() => {
                // Not fully resolved associated item `T::A::B` or `<T as Tr>::A::B`
                // or `<T>::A::B`. If `B` should be resolved in value namespace then
                // it needs to be added to the trait map.
                if ns == ValueNS {
                    let item_name = path.last().unwrap().ident;
                    let traits = self.get_traits_containing_item(item_name, ns);
                    self.trait_map.insert(id, traits);
                }
                resolution
            }
            _ => report_errors(self, None)
        };

        if let PathSource::TraitItem(..) = source {} else {
            // Avoid recording definition of `A::B` in `<T as A>::B::C`.
            self.record_def(id, resolution);
        }
        resolution
    }

    fn type_ascription_suggestion(&self,
                                  err: &mut DiagnosticBuilder,
                                  base_span: Span) {
        debug!("type_ascription_suggetion {:?}", base_span);
        let cm = self.session.source_map();
        debug!("self.current_type_ascription {:?}", self.current_type_ascription);
        if let Some(sp) = self.current_type_ascription.last() {
            let mut sp = *sp;
            loop {  // try to find the `:`, bail on first non-':'/non-whitespace
                sp = cm.next_point(sp);
                if let Ok(snippet) = cm.span_to_snippet(sp.to(cm.next_point(sp))) {
                    debug!("snippet {:?}", snippet);
                    let line_sp = cm.lookup_char_pos(sp.hi()).line;
                    let line_base_sp = cm.lookup_char_pos(base_span.lo()).line;
                    debug!("{:?} {:?}", line_sp, line_base_sp);
                    if snippet == ":" {
                        err.span_label(base_span,
                                       "expecting a type here because of type ascription");
                        if line_sp != line_base_sp {
                            err.span_suggestion_short_with_applicability(
                                sp,
                                "did you mean to use `;` here instead?",
                                ";".to_string(),
                                Applicability::MaybeIncorrect,
                            );
                        }
                        break;
                    } else if !snippet.trim().is_empty() {
                        debug!("tried to find type ascription `:` token, couldn't find it");
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    fn self_type_is_available(&mut self, span: Span) -> bool {
        let binding = self.resolve_ident_in_lexical_scope(keywords::SelfUpper.ident(),
                                                          TypeNS, None, span);
        if let Some(LexicalScopeBinding::Def(def)) = binding { def != Def::Err } else { false }
    }

    fn self_value_is_available(&mut self, self_span: Span, path_span: Span) -> bool {
        let ident = Ident::new(keywords::SelfLower.name(), self_span);
        let binding = self.resolve_ident_in_lexical_scope(ident, ValueNS, None, path_span);
        if let Some(LexicalScopeBinding::Def(def)) = binding { def != Def::Err } else { false }
    }

    // Resolve in alternative namespaces if resolution in the primary namespace fails.
    fn resolve_qpath_anywhere(&mut self,
                              id: NodeId,
                              qself: Option<&QSelf>,
                              path: &[Segment],
                              primary_ns: Namespace,
                              span: Span,
                              defer_to_typeck: bool,
                              global_by_default: bool,
                              crate_lint: CrateLint)
                              -> Option<PathResolution> {
        let mut fin_res = None;
        // FIXME: can't resolve paths in macro namespace yet, macros are
        // processed by the little special hack below.
        for (i, ns) in [primary_ns, TypeNS, ValueNS, /*MacroNS*/].iter().cloned().enumerate() {
            if i == 0 || ns != primary_ns {
                match self.resolve_qpath(id, qself, path, ns, span, global_by_default, crate_lint) {
                    // If defer_to_typeck, then resolution > no resolution,
                    // otherwise full resolution > partial resolution > no resolution.
                    Some(res) if res.unresolved_segments() == 0 || defer_to_typeck =>
                        return Some(res),
                    res => if fin_res.is_none() { fin_res = res },
                };
            }
        }
        if primary_ns != MacroNS &&
           (self.macro_names.contains(&path[0].ident.modern()) ||
            self.builtin_macros.get(&path[0].ident.name).cloned()
                               .and_then(NameBinding::macro_kind) == Some(MacroKind::Bang) ||
            self.macro_use_prelude.get(&path[0].ident.name).cloned()
                                  .and_then(NameBinding::macro_kind) == Some(MacroKind::Bang)) {
            // Return some dummy definition, it's enough for error reporting.
            return Some(
                PathResolution::new(Def::Macro(DefId::local(CRATE_DEF_INDEX), MacroKind::Bang))
            );
        }
        fin_res
    }

    /// Handles paths that may refer to associated items.
    fn resolve_qpath(&mut self,
                     id: NodeId,
                     qself: Option<&QSelf>,
                     path: &[Segment],
                     ns: Namespace,
                     span: Span,
                     global_by_default: bool,
                     crate_lint: CrateLint)
                     -> Option<PathResolution> {
        debug!(
            "resolve_qpath(id={:?}, qself={:?}, path={:?}, \
             ns={:?}, span={:?}, global_by_default={:?})",
            id,
            qself,
            path,
            ns,
            span,
            global_by_default,
        );

        if let Some(qself) = qself {
            if qself.position == 0 {
                // This is a case like `<T>::B`, where there is no
                // trait to resolve.  In that case, we leave the `B`
                // segment to be resolved by type-check.
                return Some(PathResolution::with_unresolved_segments(
                    Def::Mod(DefId::local(CRATE_DEF_INDEX)), path.len()
                ));
            }

            // Make sure `A::B` in `<T as A::B>::C` is a trait item.
            //
            // Currently, `path` names the full item (`A::B::C`, in
            // our example).  so we extract the prefix of that that is
            // the trait (the slice upto and including
            // `qself.position`). And then we recursively resolve that,
            // but with `qself` set to `None`.
            //
            // However, setting `qself` to none (but not changing the
            // span) loses the information about where this path
            // *actually* appears, so for the purposes of the crate
            // lint we pass along information that this is the trait
            // name from a fully qualified path, and this also
            // contains the full span (the `CrateLint::QPathTrait`).
            let ns = if qself.position + 1 == path.len() { ns } else { TypeNS };
            let res = self.smart_resolve_path_fragment(
                id,
                None,
                &path[..=qself.position],
                span,
                PathSource::TraitItem(ns),
                CrateLint::QPathTrait {
                    qpath_id: id,
                    qpath_span: qself.path_span,
                },
            );

            // The remaining segments (the `C` in our example) will
            // have to be resolved by type-check, since that requires doing
            // trait resolution.
            return Some(PathResolution::with_unresolved_segments(
                res.base_def(), res.unresolved_segments() + path.len() - qself.position - 1
            ));
        }

        let result = match self.resolve_path_without_parent_scope(
            &path,
            Some(ns),
            true,
            span,
            crate_lint,
        ) {
            PathResult::NonModule(path_res) => path_res,
            PathResult::Module(ModuleOrUniformRoot::Module(module)) if !module.is_normal() => {
                PathResolution::new(module.def().unwrap())
            }
            // In `a(::assoc_item)*` `a` cannot be a module. If `a` does resolve to a module we
            // don't report an error right away, but try to fallback to a primitive type.
            // So, we are still able to successfully resolve something like
            //
            // use std::u8; // bring module u8 in scope
            // fn f() -> u8 { // OK, resolves to primitive u8, not to std::u8
            //     u8::max_value() // OK, resolves to associated function <u8>::max_value,
            //                     // not to non-existent std::u8::max_value
            // }
            //
            // Such behavior is required for backward compatibility.
            // The same fallback is used when `a` resolves to nothing.
            PathResult::Module(ModuleOrUniformRoot::Module(_)) |
            PathResult::Failed(..)
                    if (ns == TypeNS || path.len() > 1) &&
                       self.primitive_type_table.primitive_types
                           .contains_key(&path[0].ident.name) => {
                let prim = self.primitive_type_table.primitive_types[&path[0].ident.name];
                PathResolution::with_unresolved_segments(Def::PrimTy(prim), path.len() - 1)
            }
            PathResult::Module(ModuleOrUniformRoot::Module(module)) =>
                PathResolution::new(module.def().unwrap()),
            PathResult::Failed(span, msg, false) => {
                resolve_error(self, span, ResolutionError::FailedToResolve(&msg));
                err_path_resolution()
            }
            PathResult::Module(..) | PathResult::Failed(..) => return None,
            PathResult::Indeterminate => bug!("indetermined path result in resolve_qpath"),
        };

        if path.len() > 1 && !global_by_default && result.base_def() != Def::Err &&
           path[0].ident.name != keywords::PathRoot.name() &&
           path[0].ident.name != keywords::DollarCrate.name() {
            let unqualified_result = {
                match self.resolve_path_without_parent_scope(
                    &[*path.last().unwrap()],
                    Some(ns),
                    false,
                    span,
                    CrateLint::No,
                ) {
                    PathResult::NonModule(path_res) => path_res.base_def(),
                    PathResult::Module(ModuleOrUniformRoot::Module(module)) =>
                        module.def().unwrap(),
                    _ => return Some(result),
                }
            };
            if result.base_def() == unqualified_result {
                let lint = lint::builtin::UNUSED_QUALIFICATIONS;
                self.session.buffer_lint(lint, id, span, "unnecessary qualification")
            }
        }

        Some(result)
    }

    fn resolve_path_without_parent_scope(
        &mut self,
        path: &[Segment],
        opt_ns: Option<Namespace>, // `None` indicates a module path in import
        record_used: bool,
        path_span: Span,
        crate_lint: CrateLint,
    ) -> PathResult<'a> {
        // Macro and import paths must have full parent scope available during resolution,
        // other paths will do okay with parent module alone.
        assert!(opt_ns != None && opt_ns != Some(MacroNS));
        let parent_scope = ParentScope { module: self.current_module, ..self.dummy_parent_scope() };
        self.resolve_path(path, opt_ns, &parent_scope, record_used, path_span, crate_lint)
    }

    fn resolve_path(
        &mut self,
        path: &[Segment],
        opt_ns: Option<Namespace>, // `None` indicates a module path in import
        parent_scope: &ParentScope<'a>,
        record_used: bool,
        path_span: Span,
        crate_lint: CrateLint,
    ) -> PathResult<'a> {
        let mut module = None;
        let mut allow_super = true;
        let mut second_binding = None;
        self.current_module = parent_scope.module;

        debug!(
            "resolve_path(path={:?}, opt_ns={:?}, record_used={:?}, \
             path_span={:?}, crate_lint={:?})",
            path,
            opt_ns,
            record_used,
            path_span,
            crate_lint,
        );

        for (i, &Segment { ident, id }) in path.iter().enumerate() {
            debug!("resolve_path ident {} {:?} {:?}", i, ident, id);
            let record_segment_def = |this: &mut Self, def| {
                if record_used {
                    if let Some(id) = id {
                        if !this.def_map.contains_key(&id) {
                            assert!(id != ast::DUMMY_NODE_ID, "Trying to resolve dummy id");
                            this.record_def(id, PathResolution::new(def));
                        }
                    }
                }
            };

            let is_last = i == path.len() - 1;
            let ns = if is_last { opt_ns.unwrap_or(TypeNS) } else { TypeNS };
            let name = ident.name;

            allow_super &= ns == TypeNS &&
                (name == keywords::SelfLower.name() ||
                 name == keywords::Super.name());

            if ns == TypeNS {
                if allow_super && name == keywords::Super.name() {
                    let mut ctxt = ident.span.ctxt().modern();
                    let self_module = match i {
                        0 => Some(self.resolve_self(&mut ctxt, self.current_module)),
                        _ => match module {
                            Some(ModuleOrUniformRoot::Module(module)) => Some(module),
                            _ => None,
                        },
                    };
                    if let Some(self_module) = self_module {
                        if let Some(parent) = self_module.parent {
                            module = Some(ModuleOrUniformRoot::Module(
                                self.resolve_self(&mut ctxt, parent)));
                            continue;
                        }
                    }
                    let msg = "there are too many initial `super`s.".to_string();
                    return PathResult::Failed(ident.span, msg, false);
                }
                if i == 0 {
                    if name == keywords::SelfLower.name() {
                        let mut ctxt = ident.span.ctxt().modern();
                        module = Some(ModuleOrUniformRoot::Module(
                            self.resolve_self(&mut ctxt, self.current_module)));
                        continue;
                    }
                    if name == keywords::Extern.name() ||
                       name == keywords::PathRoot.name() && ident.span.rust_2018() {
                        module = Some(ModuleOrUniformRoot::ExternPrelude);
                        continue;
                    }
                    if name == keywords::PathRoot.name() &&
                       ident.span.rust_2015() && self.session.rust_2018() {
                        // `::a::b` from 2015 macro on 2018 global edition
                        module = Some(ModuleOrUniformRoot::CrateRootAndExternPrelude);
                        continue;
                    }
                    if name == keywords::PathRoot.name() ||
                       name == keywords::Crate.name() ||
                       name == keywords::DollarCrate.name() {
                        // `::a::b`, `crate::a::b` or `$crate::a::b`
                        module = Some(ModuleOrUniformRoot::Module(
                            self.resolve_crate_root(ident)));
                        continue;
                    }
                }
            }

            // Report special messages for path segment keywords in wrong positions.
            if ident.is_path_segment_keyword() && i != 0 {
                let name_str = if name == keywords::PathRoot.name() {
                    "crate root".to_string()
                } else {
                    format!("`{}`", name)
                };
                let msg = if i == 1 && path[0].ident.name == keywords::PathRoot.name() {
                    format!("global paths cannot start with {}", name_str)
                } else {
                    format!("{} in paths can only be used in start position", name_str)
                };
                return PathResult::Failed(ident.span, msg, false);
            }

            let binding = if let Some(module) = module {
                self.resolve_ident_in_module(module, ident, ns, None, record_used, path_span)
            } else if opt_ns.is_none() || opt_ns == Some(MacroNS) {
                assert!(ns == TypeNS);
                let scopes = if opt_ns.is_none() { ScopeSet::Import(ns) } else { ScopeSet::Module };
                self.early_resolve_ident_in_lexical_scope(ident, scopes, parent_scope, record_used,
                                                          record_used, path_span)
            } else {
                let record_used_id =
                    if record_used { crate_lint.node_id().or(Some(CRATE_NODE_ID)) } else { None };
                match self.resolve_ident_in_lexical_scope(ident, ns, record_used_id, path_span) {
                    // we found a locally-imported or available item/module
                    Some(LexicalScopeBinding::Item(binding)) => Ok(binding),
                    // we found a local variable or type param
                    Some(LexicalScopeBinding::Def(def))
                            if opt_ns == Some(TypeNS) || opt_ns == Some(ValueNS) => {
                        record_segment_def(self, def);
                        return PathResult::NonModule(PathResolution::with_unresolved_segments(
                            def, path.len() - 1
                        ));
                    }
                    _ => Err(Determinacy::determined(record_used)),
                }
            };

            match binding {
                Ok(binding) => {
                    if i == 1 {
                        second_binding = Some(binding);
                    }
                    let def = binding.def();
                    let maybe_assoc = opt_ns != Some(MacroNS) && PathSource::Type.is_expected(def);
                    if let Some(next_module) = binding.module() {
                        module = Some(ModuleOrUniformRoot::Module(next_module));
                        record_segment_def(self, def);
                    } else if def == Def::ToolMod && i + 1 != path.len() {
                        let def = Def::NonMacroAttr(NonMacroAttrKind::Tool);
                        return PathResult::NonModule(PathResolution::new(def));
                    } else if def == Def::Err {
                        return PathResult::NonModule(err_path_resolution());
                    } else if opt_ns.is_some() && (is_last || maybe_assoc) {
                        self.lint_if_path_starts_with_module(
                            crate_lint,
                            path,
                            path_span,
                            second_binding,
                        );
                        return PathResult::NonModule(PathResolution::with_unresolved_segments(
                            def, path.len() - i - 1
                        ));
                    } else {
                        return PathResult::Failed(ident.span,
                                                  format!("not a module `{}`", ident),
                                                  is_last);
                    }
                }
                Err(Undetermined) => return PathResult::Indeterminate,
                Err(Determined) => {
                    if let Some(ModuleOrUniformRoot::Module(module)) = module {
                        if opt_ns.is_some() && !module.is_normal() {
                            return PathResult::NonModule(PathResolution::with_unresolved_segments(
                                module.def().unwrap(), path.len() - i
                            ));
                        }
                    }
                    let module_def = match module {
                        Some(ModuleOrUniformRoot::Module(module)) => module.def(),
                        _ => None,
                    };
                    let msg = if module_def == self.graph_root.def() {
                        let is_mod = |def| match def { Def::Mod(..) => true, _ => false };
                        let mut candidates =
                            self.lookup_import_candidates(ident, TypeNS, is_mod);
                        candidates.sort_by_cached_key(|c| {
                            (c.path.segments.len(), c.path.to_string())
                        });
                        if let Some(candidate) = candidates.get(0) {
                            format!("did you mean `{}`?", candidate.path)
                        } else if !ident.is_reserved() {
                            format!("maybe a missing `extern crate {};`?", ident)
                        } else {
                            // the parser will already have complained about the keyword being used
                            return PathResult::NonModule(err_path_resolution());
                        }
                    } else if i == 0 {
                        format!("use of undeclared type or module `{}`", ident)
                    } else {
                        format!("could not find `{}` in `{}`", ident, path[i - 1].ident)
                    };
                    return PathResult::Failed(ident.span, msg, is_last);
                }
            }
        }

        self.lint_if_path_starts_with_module(crate_lint, path, path_span, second_binding);

        PathResult::Module(match module {
            Some(module) => module,
            None if path.is_empty() => ModuleOrUniformRoot::CurrentScope,
            _ => span_bug!(path_span, "resolve_path: non-empty path `{:?}` has no module", path),
        })
    }

    fn lint_if_path_starts_with_module(
        &self,
        crate_lint: CrateLint,
        path: &[Segment],
        path_span: Span,
        second_binding: Option<&NameBinding>,
    ) {
        let (diag_id, diag_span) = match crate_lint {
            CrateLint::No => return,
            CrateLint::SimplePath(id) => (id, path_span),
            CrateLint::UsePath { root_id, root_span } => (root_id, root_span),
            CrateLint::QPathTrait { qpath_id, qpath_span } => (qpath_id, qpath_span),
        };

        let first_name = match path.get(0) {
            // In the 2018 edition this lint is a hard error, so nothing to do
            Some(seg) if seg.ident.span.rust_2015() && self.session.rust_2015() => seg.ident.name,
            _ => return,
        };

        // We're only interested in `use` paths which should start with
        // `{{root}}` or `extern` currently.
        if first_name != keywords::Extern.name() && first_name != keywords::PathRoot.name() {
            return
        }

        match path.get(1) {
            // If this import looks like `crate::...` it's already good
            Some(Segment { ident, .. }) if ident.name == keywords::Crate.name() => return,
            // Otherwise go below to see if it's an extern crate
            Some(_) => {}
            // If the path has length one (and it's `PathRoot` most likely)
            // then we don't know whether we're gonna be importing a crate or an
            // item in our crate. Defer this lint to elsewhere
            None => return,
        }

        // If the first element of our path was actually resolved to an
        // `ExternCrate` (also used for `crate::...`) then no need to issue a
        // warning, this looks all good!
        if let Some(binding) = second_binding {
            if let NameBindingKind::Import { directive: d, .. } = binding.kind {
                // Careful: we still want to rewrite paths from
                // renamed extern crates.
                if let ImportDirectiveSubclass::ExternCrate { source: None, .. } = d.subclass {
                    return
                }
            }
        }

        let diag = lint::builtin::BuiltinLintDiagnostics
            ::AbsPathWithModule(diag_span);
        self.session.buffer_lint_with_diagnostic(
            lint::builtin::ABSOLUTE_PATHS_NOT_STARTING_WITH_CRATE,
            diag_id, diag_span,
            "absolute paths must start with `self`, `super`, \
            `crate`, or an external crate name in the 2018 edition",
            diag);
    }

    // Resolve a local definition, potentially adjusting for closures.
    fn adjust_local_def(&mut self,
                        ns: Namespace,
                        rib_index: usize,
                        mut def: Def,
                        record_used: bool,
                        span: Span) -> Def {
        let ribs = &self.ribs[ns][rib_index + 1..];

        // An invalid forward use of a type parameter from a previous default.
        if let ForwardTyParamBanRibKind = self.ribs[ns][rib_index].kind {
            if record_used {
                resolve_error(self, span, ResolutionError::ForwardDeclaredTyParam);
            }
            assert_eq!(def, Def::Err);
            return Def::Err;
        }

        match def {
            Def::Upvar(..) => {
                span_bug!(span, "unexpected {:?} in bindings", def)
            }
            Def::Local(node_id) => {
                for rib in ribs {
                    match rib.kind {
                        NormalRibKind | ModuleRibKind(..) | MacroDefinition(..) |
                        ForwardTyParamBanRibKind => {
                            // Nothing to do. Continue.
                        }
                        ClosureRibKind(function_id) => {
                            let prev_def = def;

                            let seen = self.freevars_seen
                                           .entry(function_id)
                                           .or_default();
                            if let Some(&index) = seen.get(&node_id) {
                                def = Def::Upvar(node_id, index, function_id);
                                continue;
                            }
                            let vec = self.freevars
                                          .entry(function_id)
                                          .or_default();
                            let depth = vec.len();
                            def = Def::Upvar(node_id, depth, function_id);

                            if record_used {
                                vec.push(Freevar {
                                    def: prev_def,
                                    span,
                                });
                                seen.insert(node_id, depth);
                            }
                        }
                        ItemRibKind | TraitOrImplItemRibKind => {
                            // This was an attempt to access an upvar inside a
                            // named function item. This is not allowed, so we
                            // report an error.
                            if record_used {
                                resolve_error(self, span,
                                    ResolutionError::CannotCaptureDynamicEnvironmentInFnItem);
                            }
                            return Def::Err;
                        }
                        ConstantItemRibKind => {
                            // Still doesn't deal with upvars
                            if record_used {
                                resolve_error(self, span,
                                    ResolutionError::AttemptToUseNonConstantValueInConstant);
                            }
                            return Def::Err;
                        }
                    }
                }
            }
            Def::TyParam(..) | Def::SelfTy(..) => {
                for rib in ribs {
                    match rib.kind {
                        NormalRibKind | TraitOrImplItemRibKind | ClosureRibKind(..) |
                        ModuleRibKind(..) | MacroDefinition(..) | ForwardTyParamBanRibKind |
                        ConstantItemRibKind => {
                            // Nothing to do. Continue.
                        }
                        ItemRibKind => {
                            // This was an attempt to use a type parameter outside
                            // its scope.
                            if record_used {
                                resolve_error(self, span,
                                    ResolutionError::TypeParametersFromOuterFunction(def));
                            }
                            return Def::Err;
                        }
                    }
                }
            }
            _ => {}
        }
        def
    }

    fn lookup_assoc_candidate<FilterFn>(&mut self,
                                        ident: Ident,
                                        ns: Namespace,
                                        filter_fn: FilterFn)
                                        -> Option<AssocSuggestion>
        where FilterFn: Fn(Def) -> bool
    {
        fn extract_node_id(t: &Ty) -> Option<NodeId> {
            match t.node {
                TyKind::Path(None, _) => Some(t.id),
                TyKind::Rptr(_, ref mut_ty) => extract_node_id(&mut_ty.ty),
                // This doesn't handle the remaining `Ty` variants as they are not
                // that commonly the self_type, it might be interesting to provide
                // support for those in future.
                _ => None,
            }
        }

        // Fields are generally expected in the same contexts as locals.
        if filter_fn(Def::Local(ast::DUMMY_NODE_ID)) {
            if let Some(node_id) = self.current_self_type.as_ref().and_then(extract_node_id) {
                // Look for a field with the same name in the current self_type.
                if let Some(resolution) = self.def_map.get(&node_id) {
                    match resolution.base_def() {
                        Def::Struct(did) | Def::Union(did)
                                if resolution.unresolved_segments() == 0 => {
                            if let Some(field_names) = self.field_names.get(&did) {
                                if field_names.iter().any(|&field_name| ident.name == field_name) {
                                    return Some(AssocSuggestion::Field);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Look for associated items in the current trait.
        if let Some((module, _)) = self.current_trait_ref {
            if let Ok(binding) = self.resolve_ident_in_module(
                    ModuleOrUniformRoot::Module(module),
                    ident,
                    ns,
                    None,
                    false,
                    module.span,
                ) {
                let def = binding.def();
                if filter_fn(def) {
                    return Some(if self.has_self.contains(&def.def_id()) {
                        AssocSuggestion::MethodWithSelf
                    } else {
                        AssocSuggestion::AssocItem
                    });
                }
            }
        }

        None
    }

    fn lookup_typo_candidate<FilterFn>(&mut self,
                                       path: &[Segment],
                                       ns: Namespace,
                                       filter_fn: FilterFn,
                                       span: Span)
                                       -> Option<Symbol>
        where FilterFn: Fn(Def) -> bool
    {
        let add_module_candidates = |module: Module, names: &mut Vec<Name>| {
            for (&(ident, _), resolution) in module.resolutions.borrow().iter() {
                if let Some(binding) = resolution.borrow().binding {
                    if filter_fn(binding.def()) {
                        names.push(ident.name);
                    }
                }
            }
        };

        let mut names = Vec::new();
        if path.len() == 1 {
            // Search in lexical scope.
            // Walk backwards up the ribs in scope and collect candidates.
            for rib in self.ribs[ns].iter().rev() {
                // Locals and type parameters
                for (ident, def) in &rib.bindings {
                    if filter_fn(*def) {
                        names.push(ident.name);
                    }
                }
                // Items in scope
                if let ModuleRibKind(module) = rib.kind {
                    // Items from this module
                    add_module_candidates(module, &mut names);

                    if let ModuleKind::Block(..) = module.kind {
                        // We can see through blocks
                    } else {
                        // Items from the prelude
                        if !module.no_implicit_prelude {
                            names.extend(self.extern_prelude.iter().map(|(ident, _)| ident.name));
                            if let Some(prelude) = self.prelude {
                                add_module_candidates(prelude, &mut names);
                            }
                        }
                        break;
                    }
                }
            }
            // Add primitive types to the mix
            if filter_fn(Def::PrimTy(Bool)) {
                names.extend(
                    self.primitive_type_table.primitive_types.iter().map(|(name, _)| name)
                )
            }
        } else {
            // Search in module.
            let mod_path = &path[..path.len() - 1];
            if let PathResult::Module(module) = self.resolve_path_without_parent_scope(
                mod_path, Some(TypeNS), false, span, CrateLint::No
            ) {
                if let ModuleOrUniformRoot::Module(module) = module {
                    add_module_candidates(module, &mut names);
                }
            }
        }

        let name = path[path.len() - 1].ident.name;
        // Make sure error reporting is deterministic.
        names.sort_by_cached_key(|name| name.as_str());
        match find_best_match_for_name(names.iter(), &name.as_str(), None) {
            Some(found) if found != name => Some(found),
            _ => None,
        }
    }

    fn with_resolved_label<F>(&mut self, label: Option<Label>, id: NodeId, f: F)
        where F: FnOnce(&mut Resolver)
    {
        if let Some(label) = label {
            self.unused_labels.insert(id, label.ident.span);
            let def = Def::Label(id);
            self.with_label_rib(|this| {
                let ident = label.ident.modern_and_legacy();
                this.label_ribs.last_mut().unwrap().bindings.insert(ident, def);
                f(this);
            });
        } else {
            f(self);
        }
    }

    fn resolve_labeled_block(&mut self, label: Option<Label>, id: NodeId, block: &Block) {
        self.with_resolved_label(label, id, |this| this.visit_block(block));
    }

    fn resolve_expr(&mut self, expr: &Expr, parent: Option<&Expr>) {
        // First, record candidate traits for this expression if it could
        // result in the invocation of a method call.

        self.record_candidate_traits_for_expr_if_necessary(expr);

        // Next, resolve the node.
        match expr.node {
            ExprKind::Path(ref qself, ref path) => {
                self.smart_resolve_path(expr.id, qself.as_ref(), path, PathSource::Expr(parent));
                visit::walk_expr(self, expr);
            }

            ExprKind::Struct(ref path, ..) => {
                self.smart_resolve_path(expr.id, None, path, PathSource::Struct);
                visit::walk_expr(self, expr);
            }

            ExprKind::Break(Some(label), _) | ExprKind::Continue(Some(label)) => {
                let def = self.search_label(label.ident, |rib, ident| {
                    rib.bindings.get(&ident.modern_and_legacy()).cloned()
                });
                match def {
                    None => {
                        // Search again for close matches...
                        // Picks the first label that is "close enough", which is not necessarily
                        // the closest match
                        let close_match = self.search_label(label.ident, |rib, ident| {
                            let names = rib.bindings.iter().map(|(id, _)| &id.name);
                            find_best_match_for_name(names, &*ident.as_str(), None)
                        });
                        self.record_def(expr.id, err_path_resolution());
                        resolve_error(self,
                                      label.ident.span,
                                      ResolutionError::UndeclaredLabel(&label.ident.as_str(),
                                                                       close_match));
                    }
                    Some(Def::Label(id)) => {
                        // Since this def is a label, it is never read.
                        self.record_def(expr.id, PathResolution::new(Def::Label(id)));
                        self.unused_labels.remove(&id);
                    }
                    Some(_) => {
                        span_bug!(expr.span, "label wasn't mapped to a label def!");
                    }
                }

                // visit `break` argument if any
                visit::walk_expr(self, expr);
            }

            ExprKind::IfLet(ref pats, ref subexpression, ref if_block, ref optional_else) => {
                self.visit_expr(subexpression);

                self.ribs[ValueNS].push(Rib::new(NormalRibKind));
                let mut bindings_list = FxHashMap::default();
                for pat in pats {
                    self.resolve_pattern(pat, PatternSource::IfLet, &mut bindings_list);
                }
                // This has to happen *after* we determine which pat_idents are variants
                self.check_consistent_bindings(pats);
                self.visit_block(if_block);
                self.ribs[ValueNS].pop();

                optional_else.as_ref().map(|expr| self.visit_expr(expr));
            }

            ExprKind::Loop(ref block, label) => self.resolve_labeled_block(label, expr.id, &block),

            ExprKind::While(ref subexpression, ref block, label) => {
                self.with_resolved_label(label, expr.id, |this| {
                    this.visit_expr(subexpression);
                    this.visit_block(block);
                });
            }

            ExprKind::WhileLet(ref pats, ref subexpression, ref block, label) => {
                self.with_resolved_label(label, expr.id, |this| {
                    this.visit_expr(subexpression);
                    this.ribs[ValueNS].push(Rib::new(NormalRibKind));
                    let mut bindings_list = FxHashMap::default();
                    for pat in pats {
                        this.resolve_pattern(pat, PatternSource::WhileLet, &mut bindings_list);
                    }
                    // This has to happen *after* we determine which pat_idents are variants.
                    this.check_consistent_bindings(pats);
                    this.visit_block(block);
                    this.ribs[ValueNS].pop();
                });
            }

            ExprKind::ForLoop(ref pattern, ref subexpression, ref block, label) => {
                self.visit_expr(subexpression);
                self.ribs[ValueNS].push(Rib::new(NormalRibKind));
                self.resolve_pattern(pattern, PatternSource::For, &mut FxHashMap::default());

                self.resolve_labeled_block(label, expr.id, block);

                self.ribs[ValueNS].pop();
            }

            ExprKind::Block(ref block, label) => self.resolve_labeled_block(label, block.id, block),

            // Equivalent to `visit::walk_expr` + passing some context to children.
            ExprKind::Field(ref subexpression, _) => {
                self.resolve_expr(subexpression, Some(expr));
            }
            ExprKind::MethodCall(ref segment, ref arguments) => {
                let mut arguments = arguments.iter();
                self.resolve_expr(arguments.next().unwrap(), Some(expr));
                for argument in arguments {
                    self.resolve_expr(argument, None);
                }
                self.visit_path_segment(expr.span, segment);
            }

            ExprKind::Call(ref callee, ref arguments) => {
                self.resolve_expr(callee, Some(expr));
                for argument in arguments {
                    self.resolve_expr(argument, None);
                }
            }
            ExprKind::Type(ref type_expr, _) => {
                self.current_type_ascription.push(type_expr.span);
                visit::walk_expr(self, expr);
                self.current_type_ascription.pop();
            }
            // Resolve the body of async exprs inside the async closure to which they desugar
            ExprKind::Async(_, async_closure_id, ref block) => {
                let rib_kind = ClosureRibKind(async_closure_id);
                self.ribs[ValueNS].push(Rib::new(rib_kind));
                self.label_ribs.push(Rib::new(rib_kind));
                self.visit_block(&block);
                self.label_ribs.pop();
                self.ribs[ValueNS].pop();
            }
            // `async |x| ...` gets desugared to `|x| future_from_generator(|| ...)`, so we need to
            // resolve the arguments within the proper scopes so that usages of them inside the
            // closure are detected as upvars rather than normal closure arg usages.
            ExprKind::Closure(
                _, IsAsync::Async { closure_id: inner_closure_id, .. }, _,
                ref fn_decl, ref body, _span,
            ) => {
                let rib_kind = ClosureRibKind(expr.id);
                self.ribs[ValueNS].push(Rib::new(rib_kind));
                self.label_ribs.push(Rib::new(rib_kind));
                // Resolve arguments:
                let mut bindings_list = FxHashMap::default();
                for argument in &fn_decl.inputs {
                    self.resolve_pattern(&argument.pat, PatternSource::FnParam, &mut bindings_list);
                    self.visit_ty(&argument.ty);
                }
                // No need to resolve return type-- the outer closure return type is
                // FunctionRetTy::Default

                // Now resolve the inner closure
                {
                    let rib_kind = ClosureRibKind(inner_closure_id);
                    self.ribs[ValueNS].push(Rib::new(rib_kind));
                    self.label_ribs.push(Rib::new(rib_kind));
                    // No need to resolve arguments: the inner closure has none.
                    // Resolve the return type:
                    visit::walk_fn_ret_ty(self, &fn_decl.output);
                    // Resolve the body
                    self.visit_expr(body);
                    self.label_ribs.pop();
                    self.ribs[ValueNS].pop();
                }
                self.label_ribs.pop();
                self.ribs[ValueNS].pop();
            }
            _ => {
                visit::walk_expr(self, expr);
            }
        }
    }

    fn record_candidate_traits_for_expr_if_necessary(&mut self, expr: &Expr) {
        match expr.node {
            ExprKind::Field(_, ident) => {
                // FIXME(#6890): Even though you can't treat a method like a
                // field, we need to add any trait methods we find that match
                // the field name so that we can do some nice error reporting
                // later on in typeck.
                let traits = self.get_traits_containing_item(ident, ValueNS);
                self.trait_map.insert(expr.id, traits);
            }
            ExprKind::MethodCall(ref segment, ..) => {
                debug!("(recording candidate traits for expr) recording traits for {}",
                       expr.id);
                let traits = self.get_traits_containing_item(segment.ident, ValueNS);
                self.trait_map.insert(expr.id, traits);
            }
            _ => {
                // Nothing to do.
            }
        }
    }

    fn get_traits_containing_item(&mut self, mut ident: Ident, ns: Namespace)
                                  -> Vec<TraitCandidate> {
        debug!("(getting traits containing item) looking for '{}'", ident.name);

        let mut found_traits = Vec::new();
        // Look for the current trait.
        if let Some((module, _)) = self.current_trait_ref {
            if self.resolve_ident_in_module(
                ModuleOrUniformRoot::Module(module),
                ident,
                ns,
                None,
                false,
                module.span,
            ).is_ok() {
                let def_id = module.def_id().unwrap();
                found_traits.push(TraitCandidate { def_id: def_id, import_id: None });
            }
        }

        ident.span = ident.span.modern();
        let mut search_module = self.current_module;
        loop {
            self.get_traits_in_module_containing_item(ident, ns, search_module, &mut found_traits);
            search_module = unwrap_or!(
                self.hygienic_lexical_parent(search_module, &mut ident.span), break
            );
        }

        if let Some(prelude) = self.prelude {
            if !search_module.no_implicit_prelude {
                self.get_traits_in_module_containing_item(ident, ns, prelude, &mut found_traits);
            }
        }

        found_traits
    }

    fn get_traits_in_module_containing_item(&mut self,
                                            ident: Ident,
                                            ns: Namespace,
                                            module: Module<'a>,
                                            found_traits: &mut Vec<TraitCandidate>) {
        assert!(ns == TypeNS || ns == ValueNS);
        let mut traits = module.traits.borrow_mut();
        if traits.is_none() {
            let mut collected_traits = Vec::new();
            module.for_each_child(|name, ns, binding| {
                if ns != TypeNS { return }
                if let Def::Trait(_) = binding.def() {
                    collected_traits.push((name, binding));
                }
            });
            *traits = Some(collected_traits.into_boxed_slice());
        }

        for &(trait_name, binding) in traits.as_ref().unwrap().iter() {
            let module = binding.module().unwrap();
            let mut ident = ident;
            if ident.span.glob_adjust(module.expansion, binding.span.ctxt().modern()).is_none() {
                continue
            }
            if self.resolve_ident_in_module_unadjusted(
                ModuleOrUniformRoot::Module(module),
                ident,
                ns,
                false,
                module.span,
            ).is_ok() {
                let import_id = match binding.kind {
                    NameBindingKind::Import { directive, .. } => {
                        self.maybe_unused_trait_imports.insert(directive.id);
                        self.add_to_glob_map(directive.id, trait_name);
                        Some(directive.id)
                    }
                    _ => None,
                };
                let trait_def_id = module.def_id().unwrap();
                found_traits.push(TraitCandidate { def_id: trait_def_id, import_id: import_id });
            }
        }
    }

    fn lookup_import_candidates_from_module<FilterFn>(&mut self,
                                          lookup_ident: Ident,
                                          namespace: Namespace,
                                          start_module: &'a ModuleData<'a>,
                                          crate_name: Ident,
                                          filter_fn: FilterFn)
                                          -> Vec<ImportSuggestion>
        where FilterFn: Fn(Def) -> bool
    {
        let mut candidates = Vec::new();
        let mut seen_modules = FxHashSet::default();
        let not_local_module = crate_name != keywords::Crate.ident();
        let mut worklist = vec![(start_module, Vec::<ast::PathSegment>::new(), not_local_module)];

        while let Some((in_module,
                        path_segments,
                        in_module_is_extern)) = worklist.pop() {
            self.populate_module_if_necessary(in_module);

            // We have to visit module children in deterministic order to avoid
            // instabilities in reported imports (#43552).
            in_module.for_each_child_stable(|ident, ns, name_binding| {
                // avoid imports entirely
                if name_binding.is_import() && !name_binding.is_extern_crate() { return; }
                // avoid non-importable candidates as well
                if !name_binding.is_importable() { return; }

                // collect results based on the filter function
                if ident.name == lookup_ident.name && ns == namespace {
                    if filter_fn(name_binding.def()) {
                        // create the path
                        let mut segms = path_segments.clone();
                        if lookup_ident.span.rust_2018() {
                            // crate-local absolute paths start with `crate::` in edition 2018
                            // FIXME: may also be stabilized for Rust 2015 (Issues #45477, #44660)
                            segms.insert(
                                0, ast::PathSegment::from_ident(crate_name)
                            );
                        }

                        segms.push(ast::PathSegment::from_ident(ident));
                        let path = Path {
                            span: name_binding.span,
                            segments: segms,
                        };
                        // the entity is accessible in the following cases:
                        // 1. if it's defined in the same crate, it's always
                        // accessible (since private entities can be made public)
                        // 2. if it's defined in another crate, it's accessible
                        // only if both the module is public and the entity is
                        // declared as public (due to pruning, we don't explore
                        // outside crate private modules => no need to check this)
                        if !in_module_is_extern || name_binding.vis == ty::Visibility::Public {
                            candidates.push(ImportSuggestion { path });
                        }
                    }
                }

                // collect submodules to explore
                if let Some(module) = name_binding.module() {
                    // form the path
                    let mut path_segments = path_segments.clone();
                    path_segments.push(ast::PathSegment::from_ident(ident));

                    let is_extern_crate_that_also_appears_in_prelude =
                        name_binding.is_extern_crate() &&
                        lookup_ident.span.rust_2018();

                    let is_visible_to_user =
                        !in_module_is_extern || name_binding.vis == ty::Visibility::Public;

                    if !is_extern_crate_that_also_appears_in_prelude && is_visible_to_user {
                        // add the module to the lookup
                        let is_extern = in_module_is_extern || name_binding.is_extern_crate();
                        if seen_modules.insert(module.def_id().unwrap()) {
                            worklist.push((module, path_segments, is_extern));
                        }
                    }
                }
            })
        }

        candidates
    }

    /// When name resolution fails, this method can be used to look up candidate
    /// entities with the expected name. It allows filtering them using the
    /// supplied predicate (which should be used to only accept the types of
    /// definitions expected e.g., traits). The lookup spans across all crates.
    ///
    /// NOTE: The method does not look into imports, but this is not a problem,
    /// since we report the definitions (thus, the de-aliased imports).
    fn lookup_import_candidates<FilterFn>(&mut self,
                                          lookup_ident: Ident,
                                          namespace: Namespace,
                                          filter_fn: FilterFn)
                                          -> Vec<ImportSuggestion>
        where FilterFn: Fn(Def) -> bool
    {
        let mut suggestions = self.lookup_import_candidates_from_module(
            lookup_ident, namespace, self.graph_root, keywords::Crate.ident(), &filter_fn);

        if lookup_ident.span.rust_2018() {
            let extern_prelude_names = self.extern_prelude.clone();
            for (ident, _) in extern_prelude_names.into_iter() {
                if let Some(crate_id) = self.crate_loader.maybe_process_path_extern(ident.name,
                                                                                    ident.span) {
                    let crate_root = self.get_module(DefId {
                        krate: crate_id,
                        index: CRATE_DEF_INDEX,
                    });
                    self.populate_module_if_necessary(&crate_root);

                    suggestions.extend(self.lookup_import_candidates_from_module(
                        lookup_ident, namespace, crate_root, ident, &filter_fn));
                }
            }
        }

        suggestions
    }

    fn find_module(&mut self,
                   module_def: Def)
                   -> Option<(Module<'a>, ImportSuggestion)>
    {
        let mut result = None;
        let mut seen_modules = FxHashSet::default();
        let mut worklist = vec![(self.graph_root, Vec::new())];

        while let Some((in_module, path_segments)) = worklist.pop() {
            // abort if the module is already found
            if result.is_some() { break; }

            self.populate_module_if_necessary(in_module);

            in_module.for_each_child_stable(|ident, _, name_binding| {
                // abort if the module is already found or if name_binding is private external
                if result.is_some() || !name_binding.vis.is_visible_locally() {
                    return
                }
                if let Some(module) = name_binding.module() {
                    // form the path
                    let mut path_segments = path_segments.clone();
                    path_segments.push(ast::PathSegment::from_ident(ident));
                    if module.def() == Some(module_def) {
                        let path = Path {
                            span: name_binding.span,
                            segments: path_segments,
                        };
                        result = Some((module, ImportSuggestion { path }));
                    } else {
                        // add the module to the lookup
                        if seen_modules.insert(module.def_id().unwrap()) {
                            worklist.push((module, path_segments));
                        }
                    }
                }
            });
        }

        result
    }

    fn collect_enum_variants(&mut self, enum_def: Def) -> Option<Vec<Path>> {
        if let Def::Enum(..) = enum_def {} else {
            panic!("Non-enum def passed to collect_enum_variants: {:?}", enum_def)
        }

        self.find_module(enum_def).map(|(enum_module, enum_import_suggestion)| {
            self.populate_module_if_necessary(enum_module);

            let mut variants = Vec::new();
            enum_module.for_each_child_stable(|ident, _, name_binding| {
                if let Def::Variant(..) = name_binding.def() {
                    let mut segms = enum_import_suggestion.path.segments.clone();
                    segms.push(ast::PathSegment::from_ident(ident));
                    variants.push(Path {
                        span: name_binding.span,
                        segments: segms,
                    });
                }
            });
            variants
        })
    }

    fn record_def(&mut self, node_id: NodeId, resolution: PathResolution) {
        debug!("(recording def) recording {:?} for {}", resolution, node_id);
        if let Some(prev_res) = self.def_map.insert(node_id, resolution) {
            panic!("path resolved multiple times ({:?} before, {:?} now)", prev_res, resolution);
        }
    }

    fn resolve_visibility(&mut self, vis: &ast::Visibility) -> ty::Visibility {
        match vis.node {
            ast::VisibilityKind::Public => ty::Visibility::Public,
            ast::VisibilityKind::Crate(..) => {
                ty::Visibility::Restricted(DefId::local(CRATE_DEF_INDEX))
            }
            ast::VisibilityKind::Inherited => {
                ty::Visibility::Restricted(self.current_module.normal_ancestor_id)
            }
            ast::VisibilityKind::Restricted { ref path, id, .. } => {
                // For visibilities we are not ready to provide correct implementation of "uniform
                // paths" right now, so on 2018 edition we only allow module-relative paths for now.
                // On 2015 edition visibilities are resolved as crate-relative by default,
                // so we are prepending a root segment if necessary.
                let ident = path.segments.get(0).expect("empty path in visibility").ident;
                let crate_root = if ident.is_path_segment_keyword() {
                    None
                } else if ident.span.rust_2018() {
                    let msg = "relative paths are not supported in visibilities on 2018 edition";
                    self.session.struct_span_err(ident.span, msg)
                                .span_suggestion(path.span, "try", format!("crate::{}", path))
                                .emit();
                    return ty::Visibility::Public;
                } else {
                    let ctxt = ident.span.ctxt();
                    Some(Segment::from_ident(Ident::new(
                        keywords::PathRoot.name(), path.span.shrink_to_lo().with_ctxt(ctxt)
                    )))
                };

                let segments = crate_root.into_iter()
                    .chain(path.segments.iter().map(|seg| seg.into())).collect::<Vec<_>>();
                let def = self.smart_resolve_path_fragment(
                    id,
                    None,
                    &segments,
                    path.span,
                    PathSource::Visibility,
                    CrateLint::SimplePath(id),
                ).base_def();
                if def == Def::Err {
                    ty::Visibility::Public
                } else {
                    let vis = ty::Visibility::Restricted(def.def_id());
                    if self.is_accessible(vis) {
                        vis
                    } else {
                        self.session.span_err(path.span, "visibilities can only be restricted \
                                                          to ancestor modules");
                        ty::Visibility::Public
                    }
                }
            }
        }
    }

    fn is_accessible(&self, vis: ty::Visibility) -> bool {
        vis.is_accessible_from(self.current_module.normal_ancestor_id, self)
    }

    fn is_accessible_from(&self, vis: ty::Visibility, module: Module<'a>) -> bool {
        vis.is_accessible_from(module.normal_ancestor_id, self)
    }

    fn set_binding_parent_module(&mut self, binding: &'a NameBinding<'a>, module: Module<'a>) {
        if let Some(old_module) = self.binding_parent_modules.insert(PtrKey(binding), module) {
            if !ptr::eq(module, old_module) {
                span_bug!(binding.span, "parent module is reset for binding");
            }
        }
    }

    fn disambiguate_legacy_vs_modern(
        &self,
        legacy: &'a NameBinding<'a>,
        modern: &'a NameBinding<'a>,
    ) -> bool {
        // Some non-controversial subset of ambiguities "modern macro name" vs "macro_rules"
        // is disambiguated to mitigate regressions from macro modularization.
        // Scoping for `macro_rules` behaves like scoping for `let` at module level, in general.
        match (self.binding_parent_modules.get(&PtrKey(legacy)),
               self.binding_parent_modules.get(&PtrKey(modern))) {
            (Some(legacy), Some(modern)) =>
                legacy.normal_ancestor_id == modern.normal_ancestor_id &&
                modern.is_ancestor_of(legacy),
            _ => false,
        }
    }

    fn binding_description(&self, b: &NameBinding, ident: Ident, from_prelude: bool) -> String {
        if b.span.is_dummy() {
            let add_built_in = match b.def() {
                // These already contain the "built-in" prefix or look bad with it.
                Def::NonMacroAttr(..) | Def::PrimTy(..) | Def::ToolMod => false,
                _ => true,
            };
            let (built_in, from) = if from_prelude {
                ("", " from prelude")
            } else if b.is_extern_crate() && !b.is_import() &&
                        self.session.opts.externs.get(&ident.as_str()).is_some() {
                ("", " passed with `--extern`")
            } else if add_built_in {
                (" built-in", "")
            } else {
                ("", "")
            };

            let article = if built_in.is_empty() { b.article() } else { "a" };
            format!("{a}{built_in} {thing}{from}",
                    a = article, thing = b.descr(), built_in = built_in, from = from)
        } else {
            let introduced = if b.is_import() { "imported" } else { "defined" };
            format!("the {thing} {introduced} here",
                    thing = b.descr(), introduced = introduced)
        }
    }

    fn report_ambiguity_error(&self, ambiguity_error: &AmbiguityError) {
        let AmbiguityError { kind, ident, b1, b2, misc1, misc2 } = *ambiguity_error;
        let (b1, b2, misc1, misc2, swapped) = if b2.span.is_dummy() && !b1.span.is_dummy() {
            // We have to print the span-less alternative first, otherwise formatting looks bad.
            (b2, b1, misc2, misc1, true)
        } else {
            (b1, b2, misc1, misc2, false)
        };

        let mut err = struct_span_err!(self.session, ident.span, E0659,
                                       "`{ident}` is ambiguous ({why})",
                                       ident = ident, why = kind.descr());
        err.span_label(ident.span, "ambiguous name");

        let mut could_refer_to = |b: &NameBinding, misc: AmbiguityErrorMisc, also: &str| {
            let what = self.binding_description(b, ident, misc == AmbiguityErrorMisc::FromPrelude);
            let note_msg = format!("`{ident}` could{also} refer to {what}",
                                   ident = ident, also = also, what = what);

            let mut help_msgs = Vec::new();
            if b.is_glob_import() && (kind == AmbiguityKind::GlobVsGlob ||
                                      kind == AmbiguityKind::GlobVsExpanded ||
                                      kind == AmbiguityKind::GlobVsOuter &&
                                      swapped != also.is_empty()) {
                help_msgs.push(format!("consider adding an explicit import of \
                                        `{ident}` to disambiguate", ident = ident))
            }
            if b.is_extern_crate() && ident.span.rust_2018() {
                help_msgs.push(format!(
                    "use `::{ident}` to refer to this {thing} unambiguously",
                    ident = ident, thing = b.descr(),
                ))
            }
            if misc == AmbiguityErrorMisc::SuggestCrate {
                help_msgs.push(format!(
                    "use `crate::{ident}` to refer to this {thing} unambiguously",
                    ident = ident, thing = b.descr(),
                ))
            } else if misc == AmbiguityErrorMisc::SuggestSelf {
                help_msgs.push(format!(
                    "use `self::{ident}` to refer to this {thing} unambiguously",
                    ident = ident, thing = b.descr(),
                ))
            }

            if b.span.is_dummy() {
                err.note(&note_msg);
            } else {
                err.span_note(b.span, &note_msg);
            }
            for (i, help_msg) in help_msgs.iter().enumerate() {
                let or = if i == 0 { "" } else { "or " };
                err.help(&format!("{}{}", or, help_msg));
            }
        };

        could_refer_to(b1, misc1, "");
        could_refer_to(b2, misc2, " also");
        err.emit();
    }

    fn report_errors(&mut self, krate: &Crate) {
        self.report_with_use_injections(krate);

        for &(span_use, span_def) in &self.macro_expanded_macro_export_errors {
            let msg = "macro-expanded `macro_export` macros from the current crate \
                       cannot be referred to by absolute paths";
            self.session.buffer_lint_with_diagnostic(
                lint::builtin::MACRO_EXPANDED_MACRO_EXPORTS_ACCESSED_BY_ABSOLUTE_PATHS,
                CRATE_NODE_ID, span_use, msg,
                lint::builtin::BuiltinLintDiagnostics::
                    MacroExpandedMacroExportsAccessedByAbsolutePaths(span_def),
            );
        }

        for ambiguity_error in &self.ambiguity_errors {
            self.report_ambiguity_error(ambiguity_error);
        }

        let mut reported_spans = FxHashSet::default();
        for &PrivacyError(dedup_span, ident, binding) in &self.privacy_errors {
            if reported_spans.insert(dedup_span) {
                span_err!(self.session, ident.span, E0603, "{} `{}` is private",
                          binding.descr(), ident.name);
            }
        }
    }

    fn report_with_use_injections(&mut self, krate: &Crate) {
        for UseError { mut err, candidates, node_id, better } in self.use_injections.drain(..) {
            let (span, found_use) = UsePlacementFinder::check(krate, node_id);
            if !candidates.is_empty() {
                show_candidates(&mut err, span, &candidates, better, found_use);
            }
            err.emit();
        }
    }

    fn report_conflict<'b>(&mut self,
                       parent: Module,
                       ident: Ident,
                       ns: Namespace,
                       new_binding: &NameBinding<'b>,
                       old_binding: &NameBinding<'b>) {
        // Error on the second of two conflicting names
        if old_binding.span.lo() > new_binding.span.lo() {
            return self.report_conflict(parent, ident, ns, old_binding, new_binding);
        }

        let container = match parent.kind {
            ModuleKind::Def(Def::Mod(_), _) => "module",
            ModuleKind::Def(Def::Trait(_), _) => "trait",
            ModuleKind::Block(..) => "block",
            _ => "enum",
        };

        let old_noun = match old_binding.is_import() {
            true => "import",
            false => "definition",
        };

        let new_participle = match new_binding.is_import() {
            true => "imported",
            false => "defined",
        };

        let (name, span) = (ident.name, self.session.source_map().def_span(new_binding.span));

        if let Some(s) = self.name_already_seen.get(&name) {
            if s == &span {
                return;
            }
        }

        let old_kind = match (ns, old_binding.module()) {
            (ValueNS, _) => "value",
            (MacroNS, _) => "macro",
            (TypeNS, _) if old_binding.is_extern_crate() => "extern crate",
            (TypeNS, Some(module)) if module.is_normal() => "module",
            (TypeNS, Some(module)) if module.is_trait() => "trait",
            (TypeNS, _) => "type",
        };

        let msg = format!("the name `{}` is defined multiple times", name);

        let mut err = match (old_binding.is_extern_crate(), new_binding.is_extern_crate()) {
            (true, true) => struct_span_err!(self.session, span, E0259, "{}", msg),
            (true, _) | (_, true) => match new_binding.is_import() && old_binding.is_import() {
                true => struct_span_err!(self.session, span, E0254, "{}", msg),
                false => struct_span_err!(self.session, span, E0260, "{}", msg),
            },
            _ => match (old_binding.is_import(), new_binding.is_import()) {
                (false, false) => struct_span_err!(self.session, span, E0428, "{}", msg),
                (true, true) => struct_span_err!(self.session, span, E0252, "{}", msg),
                _ => struct_span_err!(self.session, span, E0255, "{}", msg),
            },
        };

        err.note(&format!("`{}` must be defined only once in the {} namespace of this {}",
                          name,
                          ns.descr(),
                          container));

        err.span_label(span, format!("`{}` re{} here", name, new_participle));
        if !old_binding.span.is_dummy() {
            err.span_label(self.session.source_map().def_span(old_binding.span),
                           format!("previous {} of the {} `{}` here", old_noun, old_kind, name));
        }

        // See https://github.com/rust-lang/rust/issues/32354
        if old_binding.is_import() || new_binding.is_import() {
            let binding = if new_binding.is_import() && !new_binding.span.is_dummy() {
                new_binding
            } else {
                old_binding
            };

            let cm = self.session.source_map();
            let rename_msg = "you can use `as` to change the binding name of the import";

            if let (
                Ok(snippet),
                NameBindingKind::Import { directive, ..},
                _dummy @ false,
            ) = (
                cm.span_to_snippet(binding.span),
                binding.kind.clone(),
                binding.span.is_dummy(),
            ) {
                let suggested_name = if name.as_str().chars().next().unwrap().is_uppercase() {
                    format!("Other{}", name)
                } else {
                    format!("other_{}", name)
                };

                err.span_suggestion_with_applicability(
                    binding.span,
                    &rename_msg,
                    match directive.subclass {
                        ImportDirectiveSubclass::SingleImport { type_ns_only: true, .. } =>
                            format!("self as {}", suggested_name),
                        ImportDirectiveSubclass::SingleImport { source, .. } =>
                            format!(
                                "{} as {}{}",
                                &snippet[..((source.span.hi().0 - binding.span.lo().0) as usize)],
                                suggested_name,
                                if snippet.ends_with(";") {
                                    ";"
                                } else {
                                    ""
                                }
                            ),
                        ImportDirectiveSubclass::ExternCrate { source, target, .. } =>
                            format!(
                                "extern crate {} as {};",
                                source.unwrap_or(target.name),
                                suggested_name,
                            ),
                        _ => unreachable!(),
                    },
                    Applicability::MaybeIncorrect,
                );
            } else {
                err.span_label(binding.span, rename_msg);
            }
        }

        err.emit();
        self.name_already_seen.insert(name, span);
    }

    fn extern_prelude_get(&mut self, ident: Ident, speculative: bool)
                          -> Option<&'a NameBinding<'a>> {
        if ident.is_path_segment_keyword() {
            // Make sure `self`, `super` etc produce an error when passed to here.
            return None;
        }
        self.extern_prelude.get(&ident.modern()).cloned().and_then(|entry| {
            if let Some(binding) = entry.extern_crate_item {
                Some(binding)
            } else {
                let crate_id = if !speculative {
                    self.crate_loader.process_path_extern(ident.name, ident.span)
                } else if let Some(crate_id) =
                        self.crate_loader.maybe_process_path_extern(ident.name, ident.span) {
                    crate_id
                } else {
                    return None;
                };
                let crate_root = self.get_module(DefId { krate: crate_id, index: CRATE_DEF_INDEX });
                self.populate_module_if_necessary(&crate_root);
                Some((crate_root, ty::Visibility::Public, DUMMY_SP, Mark::root())
                    .to_name_binding(self.arenas))
            }
        })
    }
}

fn is_self_type(path: &[Segment], namespace: Namespace) -> bool {
    namespace == TypeNS && path.len() == 1 && path[0].ident.name == keywords::SelfUpper.name()
}

fn is_self_value(path: &[Segment], namespace: Namespace) -> bool {
    namespace == ValueNS && path.len() == 1 && path[0].ident.name == keywords::SelfLower.name()
}

fn names_to_string(idents: &[Ident]) -> String {
    let mut result = String::new();
    for (i, ident) in idents.iter()
                            .filter(|ident| ident.name != keywords::PathRoot.name())
                            .enumerate() {
        if i > 0 {
            result.push_str("::");
        }
        result.push_str(&ident.as_str());
    }
    result
}

fn path_names_to_string(path: &Path) -> String {
    names_to_string(&path.segments.iter()
                        .map(|seg| seg.ident)
                        .collect::<Vec<_>>())
}

/// Get the stringified path for an enum from an `ImportSuggestion` for an enum variant.
fn import_candidate_to_enum_paths(suggestion: &ImportSuggestion) -> (String, String) {
    let variant_path = &suggestion.path;
    let variant_path_string = path_names_to_string(variant_path);

    let path_len = suggestion.path.segments.len();
    let enum_path = ast::Path {
        span: suggestion.path.span,
        segments: suggestion.path.segments[0..path_len - 1].to_vec(),
    };
    let enum_path_string = path_names_to_string(&enum_path);

    (variant_path_string, enum_path_string)
}


/// When an entity with a given name is not available in scope, we search for
/// entities with that name in all crates. This method allows outputting the
/// results of this search in a programmer-friendly way
fn show_candidates(err: &mut DiagnosticBuilder,
                   // This is `None` if all placement locations are inside expansions
                   span: Option<Span>,
                   candidates: &[ImportSuggestion],
                   better: bool,
                   found_use: bool) {

    // we want consistent results across executions, but candidates are produced
    // by iterating through a hash map, so make sure they are ordered:
    let mut path_strings: Vec<_> =
        candidates.into_iter().map(|c| path_names_to_string(&c.path)).collect();
    path_strings.sort();

    let better = if better { "better " } else { "" };
    let msg_diff = match path_strings.len() {
        1 => " is found in another module, you can import it",
        _ => "s are found in other modules, you can import them",
    };
    let msg = format!("possible {}candidate{} into scope", better, msg_diff);

    if let Some(span) = span {
        for candidate in &mut path_strings {
            // produce an additional newline to separate the new use statement
            // from the directly following item.
            let additional_newline = if found_use {
                ""
            } else {
                "\n"
            };
            *candidate = format!("use {};\n{}", candidate, additional_newline);
        }

        err.span_suggestions_with_applicability(
            span,
            &msg,
            path_strings.into_iter(),
            Applicability::Unspecified,
        );
    } else {
        let mut msg = msg;
        msg.push(':');
        for candidate in path_strings {
            msg.push('\n');
            msg.push_str(&candidate);
        }
    }
}

/// A somewhat inefficient routine to obtain the name of a module.
fn module_to_string(module: Module) -> Option<String> {
    let mut names = Vec::new();

    fn collect_mod(names: &mut Vec<Ident>, module: Module) {
        if let ModuleKind::Def(_, name) = module.kind {
            if let Some(parent) = module.parent {
                names.push(Ident::with_empty_ctxt(name));
                collect_mod(names, parent);
            }
        } else {
            // danger, shouldn't be ident?
            names.push(Ident::from_str("<opaque>"));
            collect_mod(names, module.parent.unwrap());
        }
    }
    collect_mod(&mut names, module);

    if names.is_empty() {
        return None;
    }
    Some(names_to_string(&names.into_iter()
                        .rev()
                        .collect::<Vec<_>>()))
}

fn err_path_resolution() -> PathResolution {
    PathResolution::new(Def::Err)
}

#[derive(PartialEq,Copy, Clone)]
pub enum MakeGlobMap {
    Yes,
    No,
}

#[derive(Copy, Clone, Debug)]
enum CrateLint {
    /// Do not issue the lint
    No,

    /// This lint applies to some random path like `impl ::foo::Bar`
    /// or whatever. In this case, we can take the span of that path.
    SimplePath(NodeId),

    /// This lint comes from a `use` statement. In this case, what we
    /// care about really is the *root* `use` statement; e.g., if we
    /// have nested things like `use a::{b, c}`, we care about the
    /// `use a` part.
    UsePath { root_id: NodeId, root_span: Span },

    /// This is the "trait item" from a fully qualified path. For example,
    /// we might be resolving  `X::Y::Z` from a path like `<T as X::Y>::Z`.
    /// The `path_span` is the span of the to the trait itself (`X::Y`).
    QPathTrait { qpath_id: NodeId, qpath_span: Span },
}

impl CrateLint {
    fn node_id(&self) -> Option<NodeId> {
        match *self {
            CrateLint::No => None,
            CrateLint::SimplePath(id) |
            CrateLint::UsePath { root_id: id, .. } |
            CrateLint::QPathTrait { qpath_id: id, .. } => Some(id),
        }
    }
}

__build_diagnostic_array! { librustc_resolve, DIAGNOSTICS }
