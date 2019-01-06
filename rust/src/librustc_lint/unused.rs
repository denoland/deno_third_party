use rustc::hir::def::Def;
use rustc::hir::def_id::DefId;
use rustc::ty;
use rustc::ty::adjustment;
use lint::{LateContext, EarlyContext, LintContext, LintArray};
use lint::{LintPass, EarlyLintPass, LateLintPass};

use syntax::ast;
use syntax::attr;
use syntax::errors::Applicability;
use syntax::feature_gate::{BUILTIN_ATTRIBUTES, AttributeType};
use syntax::print::pprust;
use syntax::symbol::keywords;
use syntax::util::parser;
use syntax_pos::Span;

use rustc::hir;

declare_lint! {
    pub UNUSED_MUST_USE,
    Warn,
    "unused result of a type flagged as #[must_use]",
    report_in_external_macro: true
}

declare_lint! {
    pub UNUSED_RESULTS,
    Allow,
    "unused result of an expression in a statement"
}

#[derive(Copy, Clone)]
pub struct UnusedResults;

impl LintPass for UnusedResults {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNUSED_MUST_USE, UNUSED_RESULTS)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnusedResults {
    fn check_stmt(&mut self, cx: &LateContext, s: &hir::Stmt) {
        let expr = match s.node {
            hir::StmtKind::Semi(ref expr, _) => &**expr,
            _ => return,
        };

        if let hir::ExprKind::Ret(..) = expr.node {
            return;
        }

        let t = cx.tables.expr_ty(&expr);
        let type_permits_lack_of_use = if t.is_unit()
            || cx.tcx.is_ty_uninhabited_from(cx.tcx.hir().get_module_parent(expr.id), t) {
            true
        } else {
            match t.sty {
                ty::Adt(def, _) => check_must_use(cx, def.did, s.span, "", ""),
                ty::Opaque(def, _) => {
                    let mut must_use = false;
                    for (predicate, _) in &cx.tcx.predicates_of(def).predicates {
                        if let ty::Predicate::Trait(ref poly_trait_predicate) = predicate {
                            let trait_ref = poly_trait_predicate.skip_binder().trait_ref;
                            if check_must_use(cx, trait_ref.def_id, s.span, "implementer of ", "") {
                                must_use = true;
                                break;
                            }
                        }
                    }
                    must_use
                }
                ty::Dynamic(binder, _) => {
                    let mut must_use = false;
                    for predicate in binder.skip_binder().iter() {
                        if let ty::ExistentialPredicate::Trait(ref trait_ref) = predicate {
                            if check_must_use(cx, trait_ref.def_id, s.span, "", " trait object") {
                                must_use = true;
                                break;
                            }
                        }
                    }
                    must_use
                }
                _ => false,
            }
        };

        let mut fn_warned = false;
        let mut op_warned = false;
        let maybe_def = match expr.node {
            hir::ExprKind::Call(ref callee, _) => {
                match callee.node {
                    hir::ExprKind::Path(ref qpath) => {
                        let def = cx.tables.qpath_def(qpath, callee.hir_id);
                        match def {
                            Def::Fn(_) | Def::Method(_) => Some(def),
                            // `Def::Local` if it was a closure, for which we
                            // do not currently support must-use linting
                            _ => None
                        }
                    },
                    _ => None
                }
            },
            hir::ExprKind::MethodCall(..) => {
                cx.tables.type_dependent_defs().get(expr.hir_id).cloned()
            },
            _ => None
        };
        if let Some(def) = maybe_def {
            let def_id = def.def_id();
            fn_warned = check_must_use(cx, def_id, s.span, "return value of ", "");
        } else if type_permits_lack_of_use {
            // We don't warn about unused unit or uninhabited types.
            // (See https://github.com/rust-lang/rust/issues/43806 for details.)
            return;
        }

        let must_use_op = match expr.node {
            // Hardcoding operators here seemed more expedient than the
            // refactoring that would be needed to look up the `#[must_use]`
            // attribute which does exist on the comparison trait methods
            hir::ExprKind::Binary(bin_op, ..)  => {
                match bin_op.node {
                    hir::BinOpKind::Eq |
                    hir::BinOpKind::Lt |
                    hir::BinOpKind::Le |
                    hir::BinOpKind::Ne |
                    hir::BinOpKind::Ge |
                    hir::BinOpKind::Gt => {
                        Some("comparison")
                    },
                    hir::BinOpKind::Add |
                    hir::BinOpKind::Sub |
                    hir::BinOpKind::Div |
                    hir::BinOpKind::Mul |
                    hir::BinOpKind::Rem => {
                        Some("arithmetic operation")
                    },
                    hir::BinOpKind::And | hir::BinOpKind::Or => {
                        Some("logical operation")
                    },
                    hir::BinOpKind::BitXor |
                    hir::BinOpKind::BitAnd |
                    hir::BinOpKind::BitOr |
                    hir::BinOpKind::Shl |
                    hir::BinOpKind::Shr => {
                        Some("bitwise operation")
                    },
                }
            },
            hir::ExprKind::Unary(..) => Some("unary operation"),
            _ => None
        };

        if let Some(must_use_op) = must_use_op {
            cx.span_lint(UNUSED_MUST_USE, expr.span,
                         &format!("unused {} that must be used", must_use_op));
            op_warned = true;
        }

        if !(type_permits_lack_of_use || fn_warned || op_warned) {
            cx.span_lint(UNUSED_RESULTS, s.span, "unused result");
        }

        fn check_must_use(
            cx: &LateContext,
            def_id: DefId,
            sp: Span,
            descr_pre_path: &str,
            descr_post_path: &str,
        ) -> bool {
            for attr in cx.tcx.get_attrs(def_id).iter() {
                if attr.check_name("must_use") {
                    let msg = format!("unused {}`{}`{} that must be used",
                        descr_pre_path, cx.tcx.item_path_str(def_id), descr_post_path);
                    let mut err = cx.struct_span_lint(UNUSED_MUST_USE, sp, &msg);
                    // check for #[must_use = "..."]
                    if let Some(note) = attr.value_str() {
                        err.note(&note.as_str());
                    }
                    err.emit();
                    return true;
                }
            }
            false
        }
    }
}

declare_lint! {
    pub PATH_STATEMENTS,
    Warn,
    "path statements with no effect"
}

#[derive(Copy, Clone)]
pub struct PathStatements;

impl LintPass for PathStatements {
    fn get_lints(&self) -> LintArray {
        lint_array!(PATH_STATEMENTS)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for PathStatements {
    fn check_stmt(&mut self, cx: &LateContext, s: &hir::Stmt) {
        if let hir::StmtKind::Semi(ref expr, _) = s.node {
            if let hir::ExprKind::Path(_) = expr.node {
                cx.span_lint(PATH_STATEMENTS, s.span, "path statement with no effect");
            }
        }
    }
}

declare_lint! {
    pub UNUSED_ATTRIBUTES,
    Warn,
    "detects attributes that were not used by the compiler"
}

#[derive(Copy, Clone)]
pub struct UnusedAttributes;

impl LintPass for UnusedAttributes {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNUSED_ATTRIBUTES)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnusedAttributes {
    fn check_attribute(&mut self, cx: &LateContext, attr: &ast::Attribute) {
        debug!("checking attribute: {:?}", attr);
        // Note that check_name() marks the attribute as used if it matches.
        for &(ref name, ty, _) in BUILTIN_ATTRIBUTES {
            match ty {
                AttributeType::Whitelisted if attr.check_name(name) => {
                    debug!("{:?} is Whitelisted", name);
                    break;
                }
                _ => (),
            }
        }

        let plugin_attributes = cx.sess().plugin_attributes.borrow_mut();
        for &(ref name, ty) in plugin_attributes.iter() {
            if ty == AttributeType::Whitelisted && attr.check_name(&name) {
                debug!("{:?} (plugin attr) is whitelisted with ty {:?}", name, ty);
                break;
            }
        }

        let name = attr.name();
        if !attr::is_used(attr) {
            debug!("Emitting warning for: {:?}", attr);
            cx.span_lint(UNUSED_ATTRIBUTES, attr.span, "unused attribute");
            // Is it a builtin attribute that must be used at the crate level?
            let known_crate = BUILTIN_ATTRIBUTES.iter()
                .find(|&&(builtin, ty, _)| name == builtin && ty == AttributeType::CrateLevel)
                .is_some();

            // Has a plugin registered this attribute as one that must be used at
            // the crate level?
            let plugin_crate = plugin_attributes.iter()
                .find(|&&(ref x, t)| name == &**x && AttributeType::CrateLevel == t)
                .is_some();
            if known_crate || plugin_crate {
                let msg = match attr.style {
                    ast::AttrStyle::Outer => {
                        "crate-level attribute should be an inner attribute: add an exclamation \
                         mark: #![foo]"
                    }
                    ast::AttrStyle::Inner => "crate-level attribute should be in the root module",
                };
                cx.span_lint(UNUSED_ATTRIBUTES, attr.span, msg);
            }
        } else {
            debug!("Attr was used: {:?}", attr);
        }
    }
}

declare_lint! {
    pub(super) UNUSED_PARENS,
    Warn,
    "`if`, `match`, `while` and `return` do not need parentheses"
}

#[derive(Copy, Clone)]
pub struct UnusedParens;

impl UnusedParens {
    fn check_unused_parens_expr(&self,
                                cx: &EarlyContext,
                                value: &ast::Expr,
                                msg: &str,
                                followed_by_block: bool) {
        if let ast::ExprKind::Paren(ref inner) = value.node {
            let necessary = followed_by_block && match inner.node {
                ast::ExprKind::Ret(_) | ast::ExprKind::Break(..) => true,
                _ => parser::contains_exterior_struct_lit(&inner),
            };
            if !necessary {
                let expr_text = if let Ok(snippet) = cx.sess().source_map()
                    .span_to_snippet(value.span) {
                        snippet
                    } else {
                        pprust::expr_to_string(value)
                    };
                Self::remove_outer_parens(cx, value.span, &expr_text, msg);
            }
        }
    }

    fn check_unused_parens_pat(&self,
                                cx: &EarlyContext,
                                value: &ast::Pat,
                                msg: &str) {
        if let ast::PatKind::Paren(_) = value.node {
            let pattern_text = if let Ok(snippet) = cx.sess().source_map()
                .span_to_snippet(value.span) {
                    snippet
                } else {
                    pprust::pat_to_string(value)
                };
            Self::remove_outer_parens(cx, value.span, &pattern_text, msg);
        }
    }

    fn remove_outer_parens(cx: &EarlyContext, span: Span, pattern: &str, msg: &str) {
        let span_msg = format!("unnecessary parentheses around {}", msg);
        let mut err = cx.struct_span_lint(UNUSED_PARENS, span, &span_msg);
        let mut ate_left_paren = false;
        let mut ate_right_paren = false;
        let parens_removed = pattern
            .trim_matches(|c| {
                match c {
                    '(' => {
                        if ate_left_paren {
                            false
                        } else {
                            ate_left_paren = true;
                            true
                        }
                    },
                    ')' => {
                        if ate_right_paren {
                            false
                        } else {
                            ate_right_paren = true;
                            true
                        }
                    },
                    _ => false,
                }
            }).to_owned();
        err.span_suggestion_short_with_applicability(
                span,
                "remove these parentheses",
                parens_removed,
                Applicability::MachineApplicable
            );
        err.emit();
    }
}

impl LintPass for UnusedParens {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNUSED_PARENS)
    }
}

impl EarlyLintPass for UnusedParens {
    fn check_expr(&mut self, cx: &EarlyContext, e: &ast::Expr) {
        use syntax::ast::ExprKind::*;
        let (value, msg, followed_by_block) = match e.node {
            If(ref cond, ..) => (cond, "`if` condition", true),
            While(ref cond, ..) => (cond, "`while` condition", true),
            IfLet(_, ref cond, ..) => (cond, "`if let` head expression", true),
            WhileLet(_, ref cond, ..) => (cond, "`while let` head expression", true),
            ForLoop(_, ref cond, ..) => (cond, "`for` head expression", true),
            Match(ref head, _) => (head, "`match` head expression", true),
            Ret(Some(ref value)) => (value, "`return` value", false),
            Assign(_, ref value) => (value, "assigned value", false),
            AssignOp(.., ref value) => (value, "assigned value", false),
            // either function/method call, or something this lint doesn't care about
            ref call_or_other => {
                let (args_to_check, call_kind) = match *call_or_other {
                    Call(_, ref args) => (&args[..], "function"),
                    // first "argument" is self (which sometimes needs parens)
                    MethodCall(_, ref args) => (&args[1..], "method"),
                    // actual catch-all arm
                    _ => {
                        return;
                    }
                };
                // Don't lint if this is a nested macro expansion: otherwise, the lint could
                // trigger in situations that macro authors shouldn't have to care about, e.g.,
                // when a parenthesized token tree matched in one macro expansion is matched as
                // an expression in another and used as a fn/method argument (Issue #47775)
                if e.span.ctxt().outer().expn_info()
                    .map_or(false, |info| info.call_site.ctxt().outer()
                            .expn_info().is_some()) {
                        return;
                }
                let msg = format!("{} argument", call_kind);
                for arg in args_to_check {
                    self.check_unused_parens_expr(cx, arg, &msg, false);
                }
                return;
            }
        };
        self.check_unused_parens_expr(cx, &value, msg, followed_by_block);
    }

    fn check_pat(&mut self, cx: &EarlyContext, p: &ast::Pat, _: &mut bool) {
        use ast::PatKind::{Paren, Range};
        // The lint visitor will visit each subpattern of `p`. We do not want to lint any range
        // pattern no matter where it occurs in the pattern. For something like `&(a..=b)`, there
        // is a recursive `check_pat` on `a` and `b`, but we will assume that if there are
        // unnecessary parens they serve a purpose of readability.
        if let Paren(ref pat) = p.node {
            match pat.node {
                Range(..) => {}
                _ => self.check_unused_parens_pat(cx, &p, "pattern")
            }
        }
    }

    fn check_stmt(&mut self, cx: &EarlyContext, s: &ast::Stmt) {
        if let ast::StmtKind::Local(ref local) = s.node {
            if let Some(ref value) = local.init {
                self.check_unused_parens_expr(cx, &value, "assigned value", false);
            }
        }
    }
}

declare_lint! {
    UNUSED_IMPORT_BRACES,
    Allow,
    "unnecessary braces around an imported item"
}

#[derive(Copy, Clone)]
pub struct UnusedImportBraces;

impl UnusedImportBraces {
    fn check_use_tree(&self, cx: &EarlyContext, use_tree: &ast::UseTree, item: &ast::Item) {
        if let ast::UseTreeKind::Nested(ref items) = use_tree.kind {
            // Recursively check nested UseTrees
            for &(ref tree, _) in items {
                self.check_use_tree(cx, tree, item);
            }

            // Trigger the lint only if there is one nested item
            if items.len() != 1 {
                return;
            }

            // Trigger the lint if the nested item is a non-self single item
            let node_ident;
            match items[0].0.kind {
                ast::UseTreeKind::Simple(rename, ..) => {
                    let orig_ident = items[0].0.prefix.segments.last().unwrap().ident;
                    if orig_ident.name == keywords::SelfLower.name() {
                        return;
                    }
                    node_ident = rename.unwrap_or(orig_ident);
                }
                ast::UseTreeKind::Glob => {
                    node_ident = ast::Ident::from_str("*");
                }
                ast::UseTreeKind::Nested(_) => {
                    return;
                }
            }

            let msg = format!("braces around {} is unnecessary", node_ident.name);
            cx.span_lint(UNUSED_IMPORT_BRACES, item.span, &msg);
        }
    }
}

impl LintPass for UnusedImportBraces {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNUSED_IMPORT_BRACES)
    }
}

impl EarlyLintPass for UnusedImportBraces {
    fn check_item(&mut self, cx: &EarlyContext, item: &ast::Item) {
        if let ast::ItemKind::Use(ref use_tree) = item.node {
            self.check_use_tree(cx, use_tree, item);
        }
    }
}

declare_lint! {
    pub(super) UNUSED_ALLOCATION,
    Warn,
    "detects unnecessary allocations that can be eliminated"
}

#[derive(Copy, Clone)]
pub struct UnusedAllocation;

impl LintPass for UnusedAllocation {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNUSED_ALLOCATION)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnusedAllocation {
    fn check_expr(&mut self, cx: &LateContext, e: &hir::Expr) {
        match e.node {
            hir::ExprKind::Box(_) => {}
            _ => return,
        }

        for adj in cx.tables.expr_adjustments(e) {
            if let adjustment::Adjust::Borrow(adjustment::AutoBorrow::Ref(_, m)) = adj.kind {
                let msg = match m {
                    adjustment::AutoBorrowMutability::Immutable =>
                        "unnecessary allocation, use & instead",
                    adjustment::AutoBorrowMutability::Mutable { .. }=>
                        "unnecessary allocation, use &mut instead"
                };
                cx.span_lint(UNUSED_ALLOCATION, e.span, msg);
            }
        }
    }
}
