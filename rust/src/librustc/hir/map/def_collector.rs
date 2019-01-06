use hir::map::definitions::*;
use hir::def_id::{CRATE_DEF_INDEX, DefIndex, DefIndexAddressSpace};
use session::CrateDisambiguator;

use syntax::ast::*;
use syntax::ext::hygiene::Mark;
use syntax::visit;
use syntax::symbol::keywords;
use syntax::symbol::Symbol;
use syntax::parse::token::{self, Token};
use syntax_pos::Span;

use hir::map::{ITEM_LIKE_SPACE, REGULAR_SPACE};

/// Creates def ids for nodes in the AST.
pub struct DefCollector<'a> {
    definitions: &'a mut Definitions,
    parent_def: Option<DefIndex>,
    expansion: Mark,
    pub visit_macro_invoc: Option<&'a mut dyn FnMut(MacroInvocationData)>,
}

pub struct MacroInvocationData {
    pub mark: Mark,
    pub def_index: DefIndex,
}

impl<'a> DefCollector<'a> {
    pub fn new(definitions: &'a mut Definitions, expansion: Mark) -> Self {
        DefCollector {
            definitions,
            expansion,
            parent_def: None,
            visit_macro_invoc: None,
        }
    }

    pub fn collect_root(&mut self,
                        crate_name: &str,
                        crate_disambiguator: CrateDisambiguator) {
        let root = self.definitions.create_root_def(crate_name,
                                                    crate_disambiguator);
        assert_eq!(root, CRATE_DEF_INDEX);
        self.parent_def = Some(root);
    }

    fn create_def(&mut self,
                  node_id: NodeId,
                  data: DefPathData,
                  address_space: DefIndexAddressSpace,
                  span: Span)
                  -> DefIndex {
        let parent_def = self.parent_def.unwrap();
        debug!("create_def(node_id={:?}, data={:?}, parent_def={:?})", node_id, data, parent_def);
        self.definitions
            .create_def_with_parent(parent_def, node_id, data, address_space, self.expansion, span)
    }

    pub fn with_parent<F: FnOnce(&mut Self)>(&mut self, parent_def: DefIndex, f: F) {
        let parent = self.parent_def;
        self.parent_def = Some(parent_def);
        f(self);
        self.parent_def = parent;
    }

    fn visit_async_fn(
        &mut self,
        id: NodeId,
        name: Name,
        span: Span,
        header: &FnHeader,
        generics: &'a Generics,
        decl: &'a FnDecl,
        body: &'a Block,
    ) {
        let (closure_id, return_impl_trait_id) = match header.asyncness {
            IsAsync::Async {
                closure_id,
                return_impl_trait_id,
            } => (closure_id, return_impl_trait_id),
            _ => unreachable!(),
        };

        // For async functions, we need to create their inner defs inside of a
        // closure to match their desugared representation.
        let fn_def_data = DefPathData::ValueNs(name.as_interned_str());
        let fn_def = self.create_def(id, fn_def_data, ITEM_LIKE_SPACE, span);
        return self.with_parent(fn_def, |this| {
            this.create_def(return_impl_trait_id, DefPathData::ImplTrait, REGULAR_SPACE, span);

            visit::walk_generics(this, generics);
            visit::walk_fn_decl(this, decl);

            let closure_def = this.create_def(closure_id,
                                              DefPathData::ClosureExpr,
                                              REGULAR_SPACE,
                                              span);
            this.with_parent(closure_def, |this| {
                visit::walk_block(this, body);
            })
        })
    }

    fn visit_macro_invoc(&mut self, id: NodeId) {
        if let Some(ref mut visit) = self.visit_macro_invoc {
            visit(MacroInvocationData {
                mark: id.placeholder_to_mark(),
                def_index: self.parent_def.unwrap(),
            })
        }
    }
}

impl<'a> visit::Visitor<'a> for DefCollector<'a> {
    fn visit_item(&mut self, i: &'a Item) {
        debug!("visit_item: {:?}", i);

        // Pick the def data. This need not be unique, but the more
        // information we encapsulate into, the better
        let def_data = match i.node {
            ItemKind::Impl(..) => DefPathData::Impl,
            ItemKind::Trait(..) => DefPathData::Trait(i.ident.as_interned_str()),
            ItemKind::Enum(..) | ItemKind::Struct(..) | ItemKind::Union(..) |
            ItemKind::TraitAlias(..) | ItemKind::Existential(..) |
            ItemKind::ExternCrate(..) | ItemKind::ForeignMod(..) | ItemKind::Ty(..) =>
                DefPathData::TypeNs(i.ident.as_interned_str()),
            ItemKind::Mod(..) if i.ident == keywords::Invalid.ident() => {
                return visit::walk_item(self, i);
            }
            ItemKind::Fn(
                ref decl,
                ref header @ FnHeader { asyncness: IsAsync::Async { .. }, .. },
                ref generics,
                ref body,
            ) => {
                return self.visit_async_fn(
                    i.id,
                    i.ident.name,
                    i.span,
                    header,
                    generics,
                    decl,
                    body,
                )
            }
            ItemKind::Mod(..) => DefPathData::Module(i.ident.as_interned_str()),
            ItemKind::Static(..) | ItemKind::Const(..) | ItemKind::Fn(..) =>
                DefPathData::ValueNs(i.ident.as_interned_str()),
            ItemKind::MacroDef(..) => DefPathData::MacroDef(i.ident.as_interned_str()),
            ItemKind::Mac(..) => return self.visit_macro_invoc(i.id),
            ItemKind::GlobalAsm(..) => DefPathData::Misc,
            ItemKind::Use(..) => {
                return visit::walk_item(self, i);
            }
        };
        let def = self.create_def(i.id, def_data, ITEM_LIKE_SPACE, i.span);

        self.with_parent(def, |this| {
            match i.node {
                ItemKind::Struct(ref struct_def, _) | ItemKind::Union(ref struct_def, _) => {
                    // If this is a tuple-like struct, register the constructor.
                    if !struct_def.is_struct() {
                        this.create_def(struct_def.id(),
                                        DefPathData::StructCtor,
                                        REGULAR_SPACE,
                                        i.span);
                    }
                }
                _ => {}
            }
            visit::walk_item(this, i);
        });
    }

    fn visit_use_tree(&mut self, use_tree: &'a UseTree, id: NodeId, _nested: bool) {
        self.create_def(id, DefPathData::Misc, ITEM_LIKE_SPACE, use_tree.span);
        visit::walk_use_tree(self, use_tree, id);
    }

    fn visit_foreign_item(&mut self, foreign_item: &'a ForeignItem) {
        if let ForeignItemKind::Macro(_) = foreign_item.node {
            return self.visit_macro_invoc(foreign_item.id);
        }

        let def = self.create_def(foreign_item.id,
                                  DefPathData::ValueNs(foreign_item.ident.as_interned_str()),
                                  REGULAR_SPACE,
                                  foreign_item.span);

        self.with_parent(def, |this| {
            visit::walk_foreign_item(this, foreign_item);
        });
    }

    fn visit_variant(&mut self, v: &'a Variant, g: &'a Generics, item_id: NodeId) {
        let def = self.create_def(v.node.data.id(),
                                  DefPathData::EnumVariant(v.node.ident.as_interned_str()),
                                  REGULAR_SPACE,
                                  v.span);
        self.with_parent(def, |this| visit::walk_variant(this, v, g, item_id));
    }

    fn visit_variant_data(&mut self, data: &'a VariantData, _: Ident,
                          _: &'a Generics, _: NodeId, _: Span) {
        for (index, field) in data.fields().iter().enumerate() {
            let name = field.ident.map(|ident| ident.name)
                .unwrap_or_else(|| Symbol::intern(&index.to_string()));
            let def = self.create_def(field.id,
                                      DefPathData::Field(name.as_interned_str()),
                                      REGULAR_SPACE,
                                      field.span);
            self.with_parent(def, |this| this.visit_struct_field(field));
        }
    }

    fn visit_generic_param(&mut self, param: &'a GenericParam) {
        let name = param.ident.as_interned_str();
        let def_path_data = match param.kind {
            GenericParamKind::Lifetime { .. } => DefPathData::LifetimeParam(name),
            GenericParamKind::Type { .. } => DefPathData::TypeParam(name),
        };
        self.create_def(param.id, def_path_data, REGULAR_SPACE, param.ident.span);

        visit::walk_generic_param(self, param);
    }

    fn visit_trait_item(&mut self, ti: &'a TraitItem) {
        let def_data = match ti.node {
            TraitItemKind::Method(..) | TraitItemKind::Const(..) =>
                DefPathData::ValueNs(ti.ident.as_interned_str()),
            TraitItemKind::Type(..) => {
                DefPathData::AssocTypeInTrait(ti.ident.as_interned_str())
            },
            TraitItemKind::Macro(..) => return self.visit_macro_invoc(ti.id),
        };

        let def = self.create_def(ti.id, def_data, ITEM_LIKE_SPACE, ti.span);
        self.with_parent(def, |this| visit::walk_trait_item(this, ti));
    }

    fn visit_impl_item(&mut self, ii: &'a ImplItem) {
        let def_data = match ii.node {
            ImplItemKind::Method(MethodSig {
                header: ref header @ FnHeader { asyncness: IsAsync::Async { .. }, .. },
                ref decl,
            }, ref body) => {
                return self.visit_async_fn(
                    ii.id,
                    ii.ident.name,
                    ii.span,
                    header,
                    &ii.generics,
                    decl,
                    body,
                )
            }
            ImplItemKind::Method(..) | ImplItemKind::Const(..) =>
                DefPathData::ValueNs(ii.ident.as_interned_str()),
            ImplItemKind::Type(..) => DefPathData::AssocTypeInImpl(ii.ident.as_interned_str()),
            ImplItemKind::Existential(..) => {
                DefPathData::AssocExistentialInImpl(ii.ident.as_interned_str())
            },
            ImplItemKind::Macro(..) => return self.visit_macro_invoc(ii.id),
        };

        let def = self.create_def(ii.id, def_data, ITEM_LIKE_SPACE, ii.span);
        self.with_parent(def, |this| visit::walk_impl_item(this, ii));
    }

    fn visit_pat(&mut self, pat: &'a Pat) {
        match pat.node {
            PatKind::Mac(..) => return self.visit_macro_invoc(pat.id),
            _ => visit::walk_pat(self, pat),
        }
    }

    fn visit_anon_const(&mut self, constant: &'a AnonConst) {
        let def = self.create_def(constant.id,
                                  DefPathData::AnonConst,
                                  REGULAR_SPACE,
                                  constant.value.span);
        self.with_parent(def, |this| visit::walk_anon_const(this, constant));
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let parent_def = self.parent_def;

        match expr.node {
            ExprKind::Mac(..) => return self.visit_macro_invoc(expr.id),
            ExprKind::Closure(_, asyncness, ..) => {
                let closure_def = self.create_def(expr.id,
                                          DefPathData::ClosureExpr,
                                          REGULAR_SPACE,
                                          expr.span);
                self.parent_def = Some(closure_def);

                // Async closures desugar to closures inside of closures, so
                // we must create two defs.
                if let IsAsync::Async { closure_id, .. } = asyncness {
                    let async_def = self.create_def(closure_id,
                                                    DefPathData::ClosureExpr,
                                                    REGULAR_SPACE,
                                                    expr.span);
                    self.parent_def = Some(async_def);
                }
            }
            ExprKind::Async(_, async_id, _) => {
                let async_def = self.create_def(async_id,
                                                DefPathData::ClosureExpr,
                                                REGULAR_SPACE,
                                                expr.span);
                self.parent_def = Some(async_def);
            }
            _ => {}
        };

        visit::walk_expr(self, expr);
        self.parent_def = parent_def;
    }

    fn visit_ty(&mut self, ty: &'a Ty) {
        match ty.node {
            TyKind::Mac(..) => return self.visit_macro_invoc(ty.id),
            TyKind::ImplTrait(node_id, _) => {
                self.create_def(node_id, DefPathData::ImplTrait, REGULAR_SPACE, ty.span);
            }
            _ => {}
        }
        visit::walk_ty(self, ty);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt.node {
            StmtKind::Mac(..) => self.visit_macro_invoc(stmt.id),
            _ => visit::walk_stmt(self, stmt),
        }
    }

    fn visit_token(&mut self, t: Token) {
        if let Token::Interpolated(nt) = t {
            if let token::NtExpr(ref expr) = nt.0 {
                if let ExprKind::Mac(..) = expr.node {
                    self.visit_macro_invoc(expr.id);
                }
            }
        }
    }
}
