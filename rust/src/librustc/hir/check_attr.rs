//! This module implements some validity checks for attributes.
//! In particular it verifies that `#[inline]` and `#[repr]` attributes are
//! attached to items that actually support them and if there are
//! conflicts between multiple such attributes attached to the same
//! item.

use hir;
use hir::intravisit::{self, Visitor, NestedVisitorMap};
use ty::TyCtxt;
use std::fmt::{self, Display};
use syntax_pos::Span;

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum Target {
    ExternCrate,
    Use,
    Static,
    Const,
    Fn,
    Closure,
    Mod,
    ForeignMod,
    GlobalAsm,
    Ty,
    Existential,
    Enum,
    Struct,
    Union,
    Trait,
    TraitAlias,
    Impl,
    Expression,
    Statement,
}

impl Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match *self {
            Target::ExternCrate => "extern crate",
            Target::Use => "use",
            Target::Static => "static item",
            Target::Const => "constant item",
            Target::Fn => "function",
            Target::Closure => "closure",
            Target::Mod => "module",
            Target::ForeignMod => "foreign module",
            Target::GlobalAsm => "global asm",
            Target::Ty => "type alias",
            Target::Existential => "existential type",
            Target::Enum => "enum",
            Target::Struct => "struct",
            Target::Union => "union",
            Target::Trait => "trait",
            Target::TraitAlias => "trait alias",
            Target::Impl => "item",
            Target::Expression => "expression",
            Target::Statement => "statement",
        })
    }
}

impl Target {
    pub(crate) fn from_item(item: &hir::Item) -> Target {
        match item.node {
            hir::ItemKind::ExternCrate(..) => Target::ExternCrate,
            hir::ItemKind::Use(..) => Target::Use,
            hir::ItemKind::Static(..) => Target::Static,
            hir::ItemKind::Const(..) => Target::Const,
            hir::ItemKind::Fn(..) => Target::Fn,
            hir::ItemKind::Mod(..) => Target::Mod,
            hir::ItemKind::ForeignMod(..) => Target::ForeignMod,
            hir::ItemKind::GlobalAsm(..) => Target::GlobalAsm,
            hir::ItemKind::Ty(..) => Target::Ty,
            hir::ItemKind::Existential(..) => Target::Existential,
            hir::ItemKind::Enum(..) => Target::Enum,
            hir::ItemKind::Struct(..) => Target::Struct,
            hir::ItemKind::Union(..) => Target::Union,
            hir::ItemKind::Trait(..) => Target::Trait,
            hir::ItemKind::TraitAlias(..) => Target::TraitAlias,
            hir::ItemKind::Impl(..) => Target::Impl,
        }
    }
}

struct CheckAttrVisitor<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
}

impl<'a, 'tcx> CheckAttrVisitor<'a, 'tcx> {
    /// Check any attribute.
    fn check_attributes(&self, item: &hir::Item, target: Target) {
        if target == Target::Fn || target == Target::Const {
            self.tcx.codegen_fn_attrs(self.tcx.hir().local_def_id(item.id));
        } else if let Some(a) = item.attrs.iter().find(|a| a.check_name("target_feature")) {
            self.tcx.sess.struct_span_err(a.span, "attribute should be applied to a function")
                .span_label(item.span, "not a function")
                .emit();
        }

        for attr in &item.attrs {
            if attr.check_name("inline") {
                self.check_inline(attr, &item.span, target)
            } else if attr.check_name("non_exhaustive") {
                self.check_non_exhaustive(attr, item, target)
            } else if attr.check_name("marker") {
                self.check_marker(attr, item, target)
            }
        }

        self.check_repr(item, target);
        self.check_used(item, target);
    }

    /// Check if an `#[inline]` is applied to a function or a closure.
    fn check_inline(&self, attr: &hir::Attribute, span: &Span, target: Target) {
        if target != Target::Fn && target != Target::Closure {
            struct_span_err!(self.tcx.sess,
                             attr.span,
                             E0518,
                             "attribute should be applied to function or closure")
                .span_label(*span, "not a function or closure")
                .emit();
        }
    }

    /// Check if the `#[non_exhaustive]` attribute on an `item` is valid.
    fn check_non_exhaustive(&self, attr: &hir::Attribute, item: &hir::Item, target: Target) {
        match target {
            Target::Struct | Target::Enum => { /* Valid */ },
            _ => {
                struct_span_err!(self.tcx.sess,
                                 attr.span,
                                 E0701,
                                 "attribute can only be applied to a struct or enum")
                    .span_label(item.span, "not a struct or enum")
                    .emit();
                return;
            }
        }

        if attr.meta_item_list().is_some() || attr.value_str().is_some() {
            struct_span_err!(self.tcx.sess,
                             attr.span,
                             E0702,
                             "attribute should be empty")
                .span_label(item.span, "not empty")
                .emit();
        }
    }

    /// Check if the `#[marker]` attribute on an `item` is valid.
    fn check_marker(&self, attr: &hir::Attribute, item: &hir::Item, target: Target) {
        match target {
            Target::Trait => { /* Valid */ },
            _ => {
                self.tcx.sess
                    .struct_span_err(attr.span, "attribute can only be applied to a trait")
                    .span_label(item.span, "not a trait")
                    .emit();
                return;
            }
        }

        if !attr.is_word() {
            self.tcx.sess
                .struct_span_err(attr.span, "attribute should be empty")
                .emit();
        }
    }

    /// Check if the `#[repr]` attributes on `item` are valid.
    fn check_repr(&self, item: &hir::Item, target: Target) {
        // Extract the names of all repr hints, e.g., [foo, bar, align] for:
        // ```
        // #[repr(foo)]
        // #[repr(bar, align(8))]
        // ```
        let hints: Vec<_> = item.attrs
            .iter()
            .filter(|attr| attr.name() == "repr")
            .filter_map(|attr| attr.meta_item_list())
            .flatten()
            .collect();

        let mut int_reprs = 0;
        let mut is_c = false;
        let mut is_simd = false;
        let mut is_transparent = false;

        for hint in &hints {
            let name = if let Some(name) = hint.name() {
                name
            } else {
                // Invalid repr hint like repr(42). We don't check for unrecognized hints here
                // (libsyntax does that), so just ignore it.
                continue;
            };

            let (article, allowed_targets) = match &*name.as_str() {
                "C" => {
                    is_c = true;
                    if target != Target::Struct &&
                            target != Target::Union &&
                            target != Target::Enum {
                                ("a", "struct, enum or union")
                    } else {
                        continue
                    }
                }
                "packed" => {
                    if target != Target::Struct &&
                            target != Target::Union {
                                ("a", "struct or union")
                    } else {
                        continue
                    }
                }
                "simd" => {
                    is_simd = true;
                    if target != Target::Struct {
                        ("a", "struct")
                    } else {
                        continue
                    }
                }
                "align" => {
                    if target != Target::Struct &&
                            target != Target::Union {
                        ("a", "struct or union")
                    } else {
                        continue
                    }
                }
                "transparent" => {
                    is_transparent = true;
                    if target != Target::Struct {
                        ("a", "struct")
                    } else {
                        continue
                    }
                }
                "i8"  | "u8"  | "i16" | "u16" |
                "i32" | "u32" | "i64" | "u64" |
                "isize" | "usize" => {
                    int_reprs += 1;
                    if target != Target::Enum {
                        ("an", "enum")
                    } else {
                        continue
                    }
                }
                _ => continue,
            };
            self.emit_repr_error(
                hint.span,
                item.span,
                &format!("attribute should be applied to {}", allowed_targets),
                &format!("not {} {}", article, allowed_targets),
            )
        }

        // Just point at all repr hints if there are any incompatibilities.
        // This is not ideal, but tracking precisely which ones are at fault is a huge hassle.
        let hint_spans = hints.iter().map(|hint| hint.span);

        // Error on repr(transparent, <anything else>).
        if is_transparent && hints.len() > 1 {
            let hint_spans: Vec<_> = hint_spans.clone().collect();
            span_err!(self.tcx.sess, hint_spans, E0692,
                      "transparent struct cannot have other repr hints");
        }
        // Warn on repr(u8, u16), repr(C, simd), and c-like-enum-repr(C, u8)
        if (int_reprs > 1)
           || (is_simd && is_c)
           || (int_reprs == 1 && is_c && is_c_like_enum(item)) {
            let hint_spans: Vec<_> = hint_spans.collect();
            span_warn!(self.tcx.sess, hint_spans, E0566,
                       "conflicting representation hints");
        }
    }

    fn emit_repr_error(
        &self,
        hint_span: Span,
        label_span: Span,
        hint_message: &str,
        label_message: &str,
    ) {
        struct_span_err!(self.tcx.sess, hint_span, E0517, "{}", hint_message)
            .span_label(label_span, label_message)
            .emit();
    }

    fn check_stmt_attributes(&self, stmt: &hir::Stmt) {
        // When checking statements ignore expressions, they will be checked later
        if let hir::StmtKind::Decl(_, _) = stmt.node {
            for attr in stmt.node.attrs() {
                if attr.check_name("inline") {
                    self.check_inline(attr, &stmt.span, Target::Statement);
                }
                if attr.check_name("repr") {
                    self.emit_repr_error(
                        attr.span,
                        stmt.span,
                        "attribute should not be applied to a statement",
                        "not a struct, enum or union",
                    );
                }
            }
        }
    }

    fn check_expr_attributes(&self, expr: &hir::Expr) {
        let target = match expr.node {
            hir::ExprKind::Closure(..) => Target::Closure,
            _ => Target::Expression,
        };
        for attr in expr.attrs.iter() {
            if attr.check_name("inline") {
                self.check_inline(attr, &expr.span, target);
            }
            if attr.check_name("repr") {
                self.emit_repr_error(
                    attr.span,
                    expr.span,
                    "attribute should not be applied to an expression",
                    "not defining a struct, enum or union",
                );
            }
        }
    }

    fn check_used(&self, item: &hir::Item, target: Target) {
        for attr in &item.attrs {
            if attr.name() == "used" && target != Target::Static {
                self.tcx.sess
                    .span_err(attr.span, "attribute must be applied to a `static` variable");
            }
        }
    }
}

impl<'a, 'tcx> Visitor<'tcx> for CheckAttrVisitor<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::OnlyBodies(&self.tcx.hir())
    }

    fn visit_item(&mut self, item: &'tcx hir::Item) {
        let target = Target::from_item(item);
        self.check_attributes(item, target);
        intravisit::walk_item(self, item)
    }


    fn visit_stmt(&mut self, stmt: &'tcx hir::Stmt) {
        self.check_stmt_attributes(stmt);
        intravisit::walk_stmt(self, stmt)
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr) {
        self.check_expr_attributes(expr);
        intravisit::walk_expr(self, expr)
    }
}

pub fn check_crate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>) {
    let mut checker = CheckAttrVisitor { tcx };
    tcx.hir().krate().visit_all_item_likes(&mut checker.as_deep_visitor());
}

fn is_c_like_enum(item: &hir::Item) -> bool {
    if let hir::ItemKind::Enum(ref def, _) = item.node {
        for variant in &def.variants {
            match variant.node.data {
                hir::VariantData::Unit(_) => { /* continue */ }
                _ => { return false; }
            }
        }
        true
    } else {
        false
    }
}
