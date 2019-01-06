// Validate AST before lowering it to HIR
//
// This pass is supposed to catch things that fit into AST data structures,
// but not permitted by the language. It runs after expansion when AST is frozen,
// so it can check for erroneous constructions produced by syntax extensions.
// This pass is supposed to perform only simple checks not requiring name resolution
// or type checking or some other kind of complex analysis.

use rustc::lint;
use rustc::session::Session;
use syntax::ast::*;
use syntax::attr;
use syntax::source_map::Spanned;
use syntax::symbol::keywords;
use syntax::ptr::P;
use syntax::visit::{self, Visitor};
use syntax_pos::Span;
use errors;
use errors::Applicability;

struct AstValidator<'a> {
    session: &'a Session,
}

impl<'a> AstValidator<'a> {
    fn err_handler(&self) -> &errors::Handler {
        &self.session.diagnostic()
    }

    fn check_lifetime(&self, ident: Ident) {
        let valid_names = [keywords::UnderscoreLifetime.name(),
                           keywords::StaticLifetime.name(),
                           keywords::Invalid.name()];
        if !valid_names.contains(&ident.name) && ident.without_first_quote().is_reserved() {
            self.err_handler().span_err(ident.span, "lifetimes cannot use keyword names");
        }
    }

    fn check_label(&self, ident: Ident) {
        if ident.without_first_quote().is_reserved() {
            self.err_handler()
                .span_err(ident.span, &format!("invalid label name `{}`", ident.name));
        }
    }

    fn invalid_non_exhaustive_attribute(&self, variant: &Variant) {
        let has_non_exhaustive = attr::contains_name(&variant.node.attrs, "non_exhaustive");
        if has_non_exhaustive {
            self.err_handler().span_err(variant.span,
                                        "#[non_exhaustive] is not yet supported on variants");
        }
    }

    fn invalid_visibility(&self, vis: &Visibility, note: Option<&str>) {
        if let VisibilityKind::Inherited = vis.node {
            return
        }

        let mut err = struct_span_err!(self.session,
                                        vis.span,
                                        E0449,
                                        "unnecessary visibility qualifier");
        if vis.node.is_pub() {
            err.span_label(vis.span, "`pub` not permitted here because it's implied");
        }
        if let Some(note) = note {
            err.note(note);
        }
        err.emit();
    }

    fn check_decl_no_pat<ReportFn: Fn(Span, bool)>(&self, decl: &FnDecl, report_err: ReportFn) {
        for arg in &decl.inputs {
            match arg.pat.node {
                PatKind::Ident(BindingMode::ByValue(Mutability::Immutable), _, None) |
                PatKind::Wild => {}
                PatKind::Ident(BindingMode::ByValue(Mutability::Mutable), _, None) =>
                    report_err(arg.pat.span, true),
                _ => report_err(arg.pat.span, false),
            }
        }
    }

    fn check_trait_fn_not_async(&self, span: Span, asyncness: IsAsync) {
        if asyncness.is_async() {
            struct_span_err!(self.session, span, E0706,
                             "trait fns cannot be declared `async`").emit()
        }
    }

    fn check_trait_fn_not_const(&self, constness: Spanned<Constness>) {
        if constness.node == Constness::Const {
            struct_span_err!(self.session, constness.span, E0379,
                             "trait fns cannot be declared const")
                .span_label(constness.span, "trait fns cannot be const")
                .emit();
        }
    }

    fn no_questions_in_bounds(&self, bounds: &GenericBounds, where_: &str, is_trait: bool) {
        for bound in bounds {
            if let GenericBound::Trait(ref poly, TraitBoundModifier::Maybe) = *bound {
                let mut err = self.err_handler().struct_span_err(poly.span,
                    &format!("`?Trait` is not permitted in {}", where_));
                if is_trait {
                    err.note(&format!("traits are `?{}` by default", poly.trait_ref.path));
                }
                err.emit();
            }
        }
    }

    /// matches '-' lit | lit (cf. parser::Parser::parse_literal_maybe_minus),
    /// or path for ranges.
    ///
    /// FIXME: do we want to allow expr -> pattern conversion to create path expressions?
    /// That means making this work:
    ///
    /// ```rust,ignore (FIXME)
    ///     struct S;
    ///     macro_rules! m {
    ///         ($a:expr) => {
    ///             let $a = S;
    ///         }
    ///     }
    ///     m!(S);
    /// ```
    fn check_expr_within_pat(&self, expr: &Expr, allow_paths: bool) {
        match expr.node {
            ExprKind::Lit(..) => {}
            ExprKind::Path(..) if allow_paths => {}
            ExprKind::Unary(UnOp::Neg, ref inner)
                if match inner.node { ExprKind::Lit(_) => true, _ => false } => {}
            _ => self.err_handler().span_err(expr.span, "arbitrary expressions aren't allowed \
                                                         in patterns")
        }
    }

    fn check_late_bound_lifetime_defs(&self, params: &[GenericParam]) {
        // Check only lifetime parameters are present and that the lifetime
        // parameters that are present have no bounds.
        let non_lt_param_spans: Vec<_> = params.iter().filter_map(|param| match param.kind {
            GenericParamKind::Lifetime { .. } => {
                if !param.bounds.is_empty() {
                    let spans: Vec<_> = param.bounds.iter().map(|b| b.span()).collect();
                    self.err_handler()
                        .span_err(spans, "lifetime bounds cannot be used in this context");
                }
                None
            }
            _ => Some(param.ident.span),
        }).collect();
        if !non_lt_param_spans.is_empty() {
            self.err_handler().span_err(non_lt_param_spans,
                "only lifetime parameters can be used in this context");
        }
    }

    /// With eRFC 2497, we need to check whether an expression is ambiguous and warn or error
    /// depending on the edition, this function handles that.
    fn while_if_let_ambiguity(&self, expr: &P<Expr>) {
        if let Some((span, op_kind)) = self.while_if_let_expr_ambiguity(&expr) {
            let mut err = self.err_handler().struct_span_err(
                span, &format!("ambiguous use of `{}`", op_kind.to_string())
            );

            err.note(
                "this will be a error until the `let_chains` feature is stabilized"
            );
            err.note(
                "see rust-lang/rust#53668 for more information"
            );

            if let Ok(snippet) = self.session.source_map().span_to_snippet(span) {
                err.span_suggestion_with_applicability(
                    span, "consider adding parentheses", format!("({})", snippet),
                    Applicability::MachineApplicable,
                );
            }

            err.emit();
        }
    }

    /// With eRFC 2497 adding if-let chains, there is a requirement that the parsing of
    /// `&&` and `||` in a if-let statement be unambiguous. This function returns a span and
    /// a `BinOpKind` (either `&&` or `||` depending on what was ambiguous) if it is determined
    /// that the current expression parsed is ambiguous and will break in future.
    fn while_if_let_expr_ambiguity(&self, expr: &P<Expr>) -> Option<(Span, BinOpKind)> {
        debug!("while_if_let_expr_ambiguity: expr.node: {:?}", expr.node);
        match &expr.node {
            ExprKind::Binary(op, _, _) if op.node == BinOpKind::And || op.node == BinOpKind::Or => {
                Some((expr.span, op.node))
            },
            ExprKind::Range(ref lhs, ref rhs, _) => {
                let lhs_ambiguous = lhs.as_ref()
                    .and_then(|lhs| self.while_if_let_expr_ambiguity(lhs));
                let rhs_ambiguous = rhs.as_ref()
                    .and_then(|rhs| self.while_if_let_expr_ambiguity(rhs));

                lhs_ambiguous.or(rhs_ambiguous)
            }
            _ => None,
        }
    }

}

impl<'a> Visitor<'a> for AstValidator<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr.node {
            ExprKind::IfLet(_, ref expr, _, _) | ExprKind::WhileLet(_, ref expr, _, _) =>
                self.while_if_let_ambiguity(&expr),
            ExprKind::InlineAsm(..) if !self.session.target.target.options.allow_asm => {
                span_err!(self.session, expr.span, E0472, "asm! is unsupported on this target");
            }
            ExprKind::ObsoleteInPlace(ref place, ref val) => {
                let mut err = self.err_handler().struct_span_err(
                    expr.span,
                    "emplacement syntax is obsolete (for now, anyway)",
                );
                err.note(
                    "for more information, see \
                     <https://github.com/rust-lang/rust/issues/27779#issuecomment-378416911>"
                );
                match val.node {
                    ExprKind::Lit(ref v) if v.node.is_numeric() => {
                        err.span_suggestion_with_applicability(
                            place.span.between(val.span),
                            "if you meant to write a comparison against a negative value, add a \
                             space in between `<` and `-`",
                            "< -".to_string(),
                            Applicability::MaybeIncorrect
                        );
                    }
                    _ => {}
                }
                err.emit();
            }
            _ => {}
        }

        visit::walk_expr(self, expr)
    }

    fn visit_ty(&mut self, ty: &'a Ty) {
        match ty.node {
            TyKind::BareFn(ref bfty) => {
                self.check_decl_no_pat(&bfty.decl, |span, _| {
                    struct_span_err!(self.session, span, E0561,
                                     "patterns aren't allowed in function pointer types").emit();
                });
                self.check_late_bound_lifetime_defs(&bfty.generic_params);
            }
            TyKind::TraitObject(ref bounds, ..) => {
                let mut any_lifetime_bounds = false;
                for bound in bounds {
                    if let GenericBound::Outlives(ref lifetime) = *bound {
                        if any_lifetime_bounds {
                            span_err!(self.session, lifetime.ident.span, E0226,
                                      "only a single explicit lifetime bound is permitted");
                            break;
                        }
                        any_lifetime_bounds = true;
                    }
                }
                self.no_questions_in_bounds(bounds, "trait object types", false);
            }
            TyKind::ImplTrait(_, ref bounds) => {
                if !bounds.iter()
                          .any(|b| if let GenericBound::Trait(..) = *b { true } else { false }) {
                    self.err_handler().span_err(ty.span, "at least one trait must be specified");
                }
            }
            _ => {}
        }

        visit::walk_ty(self, ty)
    }

    fn visit_label(&mut self, label: &'a Label) {
        self.check_label(label.ident);
        visit::walk_label(self, label);
    }

    fn visit_lifetime(&mut self, lifetime: &'a Lifetime) {
        self.check_lifetime(lifetime.ident);
        visit::walk_lifetime(self, lifetime);
    }

    fn visit_item(&mut self, item: &'a Item) {
        match item.node {
            ItemKind::Impl(unsafety, polarity, _, _, Some(..), ref ty, ref impl_items) => {
                self.invalid_visibility(&item.vis, None);
                if let TyKind::Err = ty.node {
                    self.err_handler()
                        .struct_span_err(item.span, "`impl Trait for .. {}` is an obsolete syntax")
                        .help("use `auto trait Trait {}` instead").emit();
                }
                if unsafety == Unsafety::Unsafe && polarity == ImplPolarity::Negative {
                    span_err!(self.session, item.span, E0198, "negative impls cannot be unsafe");
                }
                for impl_item in impl_items {
                    self.invalid_visibility(&impl_item.vis, None);
                    if let ImplItemKind::Method(ref sig, _) = impl_item.node {
                        self.check_trait_fn_not_const(sig.header.constness);
                        self.check_trait_fn_not_async(impl_item.span, sig.header.asyncness);
                    }
                }
            }
            ItemKind::Impl(unsafety, polarity, defaultness, _, None, _, _) => {
                self.invalid_visibility(&item.vis,
                                        Some("place qualifiers on individual impl items instead"));
                if unsafety == Unsafety::Unsafe {
                    span_err!(self.session, item.span, E0197, "inherent impls cannot be unsafe");
                }
                if polarity == ImplPolarity::Negative {
                    self.err_handler().span_err(item.span, "inherent impls cannot be negative");
                }
                if defaultness == Defaultness::Default {
                    self.err_handler()
                        .struct_span_err(item.span, "inherent impls cannot be default")
                        .note("only trait implementations may be annotated with default").emit();
                }
            }
            ItemKind::ForeignMod(..) => {
                self.invalid_visibility(
                    &item.vis,
                    Some("place qualifiers on individual foreign items instead"),
                );
            }
            ItemKind::Enum(ref def, _) => {
                for variant in &def.variants {
                    self.invalid_non_exhaustive_attribute(variant);
                    for field in variant.node.data.fields() {
                        self.invalid_visibility(&field.vis, None);
                    }
                }
            }
            ItemKind::Trait(is_auto, _, ref generics, ref bounds, ref trait_items) => {
                if is_auto == IsAuto::Yes {
                    // Auto traits cannot have generics, super traits nor contain items.
                    if !generics.params.is_empty() {
                        struct_span_err!(self.session, item.span, E0567,
                                        "auto traits cannot have generic parameters").emit();
                    }
                    if !bounds.is_empty() {
                        struct_span_err!(self.session, item.span, E0568,
                                        "auto traits cannot have super traits").emit();
                    }
                    if !trait_items.is_empty() {
                        struct_span_err!(self.session, item.span, E0380,
                                "auto traits cannot have methods or associated items").emit();
                    }
                }
                self.no_questions_in_bounds(bounds, "supertraits", true);
                for trait_item in trait_items {
                    if let TraitItemKind::Method(ref sig, ref block) = trait_item.node {
                        self.check_trait_fn_not_async(trait_item.span, sig.header.asyncness);
                        self.check_trait_fn_not_const(sig.header.constness);
                        if block.is_none() {
                            self.check_decl_no_pat(&sig.decl, |span, mut_ident| {
                                if mut_ident {
                                    self.session.buffer_lint(
                                        lint::builtin::PATTERNS_IN_FNS_WITHOUT_BODY,
                                        trait_item.id, span,
                                        "patterns aren't allowed in methods without bodies");
                                } else {
                                    struct_span_err!(self.session, span, E0642,
                                        "patterns aren't allowed in methods without bodies").emit();
                                }
                            });
                        }
                    }
                }
            }
            ItemKind::Mod(_) => {
                // Ensure that `path` attributes on modules are recorded as used (cf. issue #35584).
                attr::first_attr_value_str_by_name(&item.attrs, "path");
                if attr::contains_name(&item.attrs, "warn_directory_ownership") {
                    let lint = lint::builtin::LEGACY_DIRECTORY_OWNERSHIP;
                    let msg = "cannot declare a new module at this location";
                    self.session.buffer_lint(lint, item.id, item.span, msg);
                }
            }
            ItemKind::Union(ref vdata, _) => {
                if !vdata.is_struct() {
                    self.err_handler().span_err(item.span,
                                                "tuple and unit unions are not permitted");
                }
                if vdata.fields().is_empty() {
                    self.err_handler().span_err(item.span,
                                                "unions cannot have zero fields");
                }
            }
            _ => {}
        }

        visit::walk_item(self, item)
    }

    fn visit_foreign_item(&mut self, fi: &'a ForeignItem) {
        match fi.node {
            ForeignItemKind::Fn(ref decl, _) => {
                self.check_decl_no_pat(decl, |span, _| {
                    struct_span_err!(self.session, span, E0130,
                                     "patterns aren't allowed in foreign function declarations")
                        .span_label(span, "pattern not allowed in foreign function").emit();
                });
            }
            ForeignItemKind::Static(..) | ForeignItemKind::Ty | ForeignItemKind::Macro(..) => {}
        }

        visit::walk_foreign_item(self, fi)
    }

    fn visit_generics(&mut self, generics: &'a Generics) {
        let mut seen_non_lifetime_param = false;
        let mut seen_default = None;
        for param in &generics.params {
            match (&param.kind, seen_non_lifetime_param) {
                (GenericParamKind::Lifetime { .. }, true) => {
                    self.err_handler()
                        .span_err(param.ident.span, "lifetime parameters must be leading");
                },
                (GenericParamKind::Lifetime { .. }, false) => {}
                (GenericParamKind::Type { ref default, .. }, _) => {
                    seen_non_lifetime_param = true;
                    if default.is_some() {
                        seen_default = Some(param.ident.span);
                    } else if let Some(span) = seen_default {
                        self.err_handler()
                            .span_err(span, "type parameters with a default must be trailing");
                        break;
                    }
                }
            }
        }
        for predicate in &generics.where_clause.predicates {
            if let WherePredicate::EqPredicate(ref predicate) = *predicate {
                self.err_handler().span_err(predicate.span, "equality constraints are not yet \
                                                             supported in where clauses (#20041)");
            }
        }
        visit::walk_generics(self, generics)
    }

    fn visit_generic_param(&mut self, param: &'a GenericParam) {
        if let GenericParamKind::Lifetime { .. } = param.kind {
            self.check_lifetime(param.ident);
        }
        visit::walk_generic_param(self, param);
    }

    fn visit_pat(&mut self, pat: &'a Pat) {
        match pat.node {
            PatKind::Lit(ref expr) => {
                self.check_expr_within_pat(expr, false);
            }
            PatKind::Range(ref start, ref end, _) => {
                self.check_expr_within_pat(start, true);
                self.check_expr_within_pat(end, true);
            }
            _ => {}
        }

        visit::walk_pat(self, pat)
    }

    fn visit_where_predicate(&mut self, p: &'a WherePredicate) {
        if let &WherePredicate::BoundPredicate(ref bound_predicate) = p {
            // A type binding, eg `for<'c> Foo: Send+Clone+'c`
            self.check_late_bound_lifetime_defs(&bound_predicate.bound_generic_params);
        }
        visit::walk_where_predicate(self, p);
    }

    fn visit_poly_trait_ref(&mut self, t: &'a PolyTraitRef, m: &'a TraitBoundModifier) {
        self.check_late_bound_lifetime_defs(&t.bound_generic_params);
        visit::walk_poly_trait_ref(self, t, m);
    }

    fn visit_mac(&mut self, mac: &Spanned<Mac_>) {
        // when a new macro kind is added but the author forgets to set it up for expansion
        // because that's the only part that won't cause a compiler error
        self.session.diagnostic()
            .span_bug(mac.span, "macro invocation missed in expansion; did you forget to override \
                                 the relevant `fold_*()` method in `PlaceholderExpander`?");
    }
}

// Bans nested `impl Trait`, e.g., `impl Into<impl Debug>`.
// Nested `impl Trait` _is_ allowed in associated type position,
// e.g `impl Iterator<Item=impl Debug>`
struct NestedImplTraitVisitor<'a> {
    session: &'a Session,
    outer_impl_trait: Option<Span>,
}

impl<'a> NestedImplTraitVisitor<'a> {
    fn with_impl_trait<F>(&mut self, outer_impl_trait: Option<Span>, f: F)
        where F: FnOnce(&mut NestedImplTraitVisitor<'a>)
    {
        let old_outer_impl_trait = self.outer_impl_trait;
        self.outer_impl_trait = outer_impl_trait;
        f(self);
        self.outer_impl_trait = old_outer_impl_trait;
    }
}


impl<'a> Visitor<'a> for NestedImplTraitVisitor<'a> {
    fn visit_ty(&mut self, t: &'a Ty) {
        if let TyKind::ImplTrait(..) = t.node {
            if let Some(outer_impl_trait) = self.outer_impl_trait {
                struct_span_err!(self.session, t.span, E0666,
                                 "nested `impl Trait` is not allowed")
                    .span_label(outer_impl_trait, "outer `impl Trait`")
                    .span_label(t.span, "nested `impl Trait` here")
                    .emit();

            }
            self.with_impl_trait(Some(t.span), |this| visit::walk_ty(this, t));
        } else {
            visit::walk_ty(self, t);
        }
    }
    fn visit_generic_args(&mut self, _: Span, generic_args: &'a GenericArgs) {
        match *generic_args {
            GenericArgs::AngleBracketed(ref data) => {
                for arg in &data.args {
                    self.visit_generic_arg(arg)
                }
                for type_binding in &data.bindings {
                    // Type bindings such as `Item=impl Debug` in `Iterator<Item=Debug>`
                    // are allowed to contain nested `impl Trait`.
                    self.with_impl_trait(None, |this| visit::walk_ty(this, &type_binding.ty));
                }
            }
            GenericArgs::Parenthesized(ref data) => {
                for type_ in &data.inputs {
                    self.visit_ty(type_);
                }
                if let Some(ref type_) = data.output {
                    // `-> Foo` syntax is essentially an associated type binding,
                    // so it is also allowed to contain nested `impl Trait`.
                    self.with_impl_trait(None, |this| visit::walk_ty(this, type_));
                }
            }
        }
    }

    fn visit_mac(&mut self, _mac: &Spanned<Mac_>) {
        // covered in AstValidator
    }
}

// Bans `impl Trait` in path projections like `<impl Iterator>::Item` or `Foo::Bar<impl Trait>`.
struct ImplTraitProjectionVisitor<'a> {
    session: &'a Session,
    is_banned: bool,
}

impl<'a> ImplTraitProjectionVisitor<'a> {
    fn with_ban<F>(&mut self, f: F)
        where F: FnOnce(&mut ImplTraitProjectionVisitor<'a>)
    {
        let old_is_banned = self.is_banned;
        self.is_banned = true;
        f(self);
        self.is_banned = old_is_banned;
    }
}

impl<'a> Visitor<'a> for ImplTraitProjectionVisitor<'a> {
    fn visit_ty(&mut self, t: &'a Ty) {
        match t.node {
            TyKind::ImplTrait(..) => {
                if self.is_banned {
                    struct_span_err!(self.session, t.span, E0667,
                        "`impl Trait` is not allowed in path parameters").emit();
                }
            }
            TyKind::Path(ref qself, ref path) => {
                // We allow these:
                //  - `Option<impl Trait>`
                //  - `option::Option<impl Trait>`
                //  - `option::Option<T>::Foo<impl Trait>
                //
                // But not these:
                //  - `<impl Trait>::Foo`
                //  - `option::Option<impl Trait>::Foo`.
                //
                // To implement this, we disallow `impl Trait` from `qself`
                // (for cases like `<impl Trait>::Foo>`)
                // but we allow `impl Trait` in `GenericArgs`
                // iff there are no more PathSegments.
                if let Some(ref qself) = *qself {
                    // `impl Trait` in `qself` is always illegal
                    self.with_ban(|this| this.visit_ty(&qself.ty));
                }

                for (i, segment) in path.segments.iter().enumerate() {
                    // Allow `impl Trait` iff we're on the final path segment
                    if i == path.segments.len() - 1 {
                        visit::walk_path_segment(self, path.span, segment);
                    } else {
                        self.with_ban(|this|
                            visit::walk_path_segment(this, path.span, segment));
                    }
                }
            }
            _ => visit::walk_ty(self, t),
        }
    }

    fn visit_mac(&mut self, _mac: &Spanned<Mac_>) {
        // covered in AstValidator
    }
}

pub fn check_crate(session: &Session, krate: &Crate) {
    visit::walk_crate(
        &mut NestedImplTraitVisitor {
            session,
            outer_impl_trait: None,
        }, krate);

    visit::walk_crate(
        &mut ImplTraitProjectionVisitor {
            session,
            is_banned: false,
        }, krate);

    visit::walk_crate(&mut AstValidator { session }, krate)
}
