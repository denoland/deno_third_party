use rustc::middle::allocator::AllocatorKind;
use rustc_errors;
use smallvec::SmallVec;
use syntax::{
    ast::{
        self, Arg, Attribute, Crate, Expr, FnHeader, Generics, Ident, Item, ItemKind,
        Mac, Mod, Mutability, Ty, TyKind, Unsafety, VisibilityKind,
    },
    attr,
    source_map::{
        respan, ExpnInfo, MacroAttribute,
    },
    ext::{
        base::{ExtCtxt, Resolver},
        build::AstBuilder,
        expand::ExpansionConfig,
        hygiene::{self, Mark, SyntaxContext},
    },
    fold::{self, Folder},
    parse::ParseSess,
    ptr::P,
    symbol::Symbol
};
use syntax_pos::Span;

use {AllocatorMethod, AllocatorTy, ALLOCATOR_METHODS};

pub fn modify(
    sess: &ParseSess,
    resolver: &mut dyn Resolver,
    krate: Crate,
    crate_name: String,
    handler: &rustc_errors::Handler,
) -> ast::Crate {
    ExpandAllocatorDirectives {
        handler,
        sess,
        resolver,
        found: false,
        crate_name: Some(crate_name),
        in_submod: -1, // -1 to account for the "root" module
    }.fold_crate(krate)
}

struct ExpandAllocatorDirectives<'a> {
    found: bool,
    handler: &'a rustc_errors::Handler,
    sess: &'a ParseSess,
    resolver: &'a mut dyn Resolver,
    crate_name: Option<String>,

    // For now, we disallow `global_allocator` in submodules because hygiene is hard. Keep track of
    // whether we are in a submodule or not. If `in_submod > 0` we are in a submodule.
    in_submod: isize,
}

impl<'a> Folder for ExpandAllocatorDirectives<'a> {
    fn fold_item(&mut self, item: P<Item>) -> SmallVec<[P<Item>; 1]> {
        debug!("in submodule {}", self.in_submod);

        let name = if attr::contains_name(&item.attrs, "global_allocator") {
            "global_allocator"
        } else {
            return fold::noop_fold_item(item, self);
        };
        match item.node {
            ItemKind::Static(..) => {}
            _ => {
                self.handler
                    .span_err(item.span, "allocators must be statics");
                return smallvec![item];
            }
        }

        if self.in_submod > 0 {
            self.handler
                .span_err(item.span, "`global_allocator` cannot be used in submodules");
            return smallvec![item];
        }

        if self.found {
            self.handler
                .span_err(item.span, "cannot define more than one #[global_allocator]");
            return smallvec![item];
        }
        self.found = true;

        // Create a fresh Mark for the new macro expansion we are about to do
        let mark = Mark::fresh(Mark::root());
        mark.set_expn_info(ExpnInfo {
            call_site: item.span, // use the call site of the static
            def_site: None,
            format: MacroAttribute(Symbol::intern(name)),
            allow_internal_unstable: true,
            allow_internal_unsafe: false,
            local_inner_macros: false,
            edition: hygiene::default_edition(),
        });

        // Tie the span to the macro expansion info we just created
        let span = item.span.with_ctxt(SyntaxContext::empty().apply_mark(mark));

        // Create an expansion config
        let ecfg = ExpansionConfig::default(self.crate_name.take().unwrap());

        // Generate a bunch of new items using the AllocFnFactory
        let mut f = AllocFnFactory {
            span,
            kind: AllocatorKind::Global,
            global: item.ident,
            core: Ident::from_str("core"),
            cx: ExtCtxt::new(self.sess, ecfg, self.resolver),
        };

        // We will generate a new submodule. To `use` the static from that module, we need to get
        // the `super::...` path.
        let super_path = f.cx.path(f.span, vec![Ident::from_str("super"), f.global]);

        // Generate the items in the submodule
        let mut items = vec![
            // import `core` to use allocators
            f.cx.item_extern_crate(f.span, f.core),
            // `use` the `global_allocator` in `super`
            f.cx.item_use_simple(
                f.span,
                respan(f.span.shrink_to_lo(), VisibilityKind::Inherited),
                super_path,
            ),
        ];

        // Add the allocator methods to the submodule
        items.extend(
            ALLOCATOR_METHODS
                .iter()
                .map(|method| f.allocator_fn(method)),
        );

        // Generate the submodule itself
        let name = f.kind.fn_name("allocator_abi");
        let allocator_abi = Ident::with_empty_ctxt(Symbol::gensym(&name));
        let module = f.cx.item_mod(span, span, allocator_abi, Vec::new(), items);
        let module = f.cx.monotonic_expander().fold_item(module).pop().unwrap();

        // Return the item and new submodule
        smallvec![item, module]
    }

    // If we enter a submodule, take note.
    fn fold_mod(&mut self, m: Mod) -> Mod {
        debug!("enter submodule");
        self.in_submod += 1;
        let ret = fold::noop_fold_mod(m, self);
        self.in_submod -= 1;
        debug!("exit submodule");
        ret
    }

    // `fold_mac` is disabled by default. Enable it here.
    fn fold_mac(&mut self, mac: Mac) -> Mac {
        fold::noop_fold_mac(mac, self)
    }
}

struct AllocFnFactory<'a> {
    span: Span,
    kind: AllocatorKind,
    global: Ident,
    core: Ident,
    cx: ExtCtxt<'a>,
}

impl<'a> AllocFnFactory<'a> {
    fn allocator_fn(&self, method: &AllocatorMethod) -> P<Item> {
        let mut abi_args = Vec::new();
        let mut i = 0;
        let ref mut mk = || {
            let name = Ident::from_str(&format!("arg{}", i));
            i += 1;
            name
        };
        let args = method
            .inputs
            .iter()
            .map(|ty| self.arg_ty(ty, &mut abi_args, mk))
            .collect();
        let result = self.call_allocator(method.name, args);
        let (output_ty, output_expr) = self.ret_ty(&method.output, result);
        let kind = ItemKind::Fn(
            self.cx.fn_decl(abi_args, ast::FunctionRetTy::Ty(output_ty)),
            FnHeader {
                unsafety: Unsafety::Unsafe,
                ..FnHeader::default()
            },
            Generics::default(),
            self.cx.block_expr(output_expr),
        );
        self.cx.item(
            self.span,
            Ident::from_str(&self.kind.fn_name(method.name)),
            self.attrs(),
            kind,
        )
    }

    fn call_allocator(&self, method: &str, mut args: Vec<P<Expr>>) -> P<Expr> {
        let method = self.cx.path(
            self.span,
            vec![
                self.core,
                Ident::from_str("alloc"),
                Ident::from_str("GlobalAlloc"),
                Ident::from_str(method),
            ],
        );
        let method = self.cx.expr_path(method);
        let allocator = self.cx.path_ident(self.span, self.global);
        let allocator = self.cx.expr_path(allocator);
        let allocator = self.cx.expr_addr_of(self.span, allocator);
        args.insert(0, allocator);

        self.cx.expr_call(self.span, method, args)
    }

    fn attrs(&self) -> Vec<Attribute> {
        let special = Symbol::intern("rustc_std_internal_symbol");
        let special = self.cx.meta_word(self.span, special);
        vec![self.cx.attribute(self.span, special)]
    }

    fn arg_ty(
        &self,
        ty: &AllocatorTy,
        args: &mut Vec<Arg>,
        ident: &mut dyn FnMut() -> Ident,
    ) -> P<Expr> {
        match *ty {
            AllocatorTy::Layout => {
                let usize = self.cx.path_ident(self.span, Ident::from_str("usize"));
                let ty_usize = self.cx.ty_path(usize);
                let size = ident();
                let align = ident();
                args.push(self.cx.arg(self.span, size, ty_usize.clone()));
                args.push(self.cx.arg(self.span, align, ty_usize));

                let layout_new = self.cx.path(
                    self.span,
                    vec![
                        self.core,
                        Ident::from_str("alloc"),
                        Ident::from_str("Layout"),
                        Ident::from_str("from_size_align_unchecked"),
                    ],
                );
                let layout_new = self.cx.expr_path(layout_new);
                let size = self.cx.expr_ident(self.span, size);
                let align = self.cx.expr_ident(self.span, align);
                let layout = self.cx.expr_call(self.span, layout_new, vec![size, align]);
                layout
            }

            AllocatorTy::Ptr => {
                let ident = ident();
                args.push(self.cx.arg(self.span, ident, self.ptr_u8()));
                let arg = self.cx.expr_ident(self.span, ident);
                self.cx.expr_cast(self.span, arg, self.ptr_u8())
            }

            AllocatorTy::Usize => {
                let ident = ident();
                args.push(self.cx.arg(self.span, ident, self.usize()));
                self.cx.expr_ident(self.span, ident)
            }

            AllocatorTy::ResultPtr | AllocatorTy::Unit => {
                panic!("can't convert AllocatorTy to an argument")
            }
        }
    }

    fn ret_ty(&self, ty: &AllocatorTy, expr: P<Expr>) -> (P<Ty>, P<Expr>) {
        match *ty {
            AllocatorTy::ResultPtr => {
                // We're creating:
                //
                //      #expr as *mut u8

                let expr = self.cx.expr_cast(self.span, expr, self.ptr_u8());
                (self.ptr_u8(), expr)
            }

            AllocatorTy::Unit => (self.cx.ty(self.span, TyKind::Tup(Vec::new())), expr),

            AllocatorTy::Layout | AllocatorTy::Usize | AllocatorTy::Ptr => {
                panic!("can't convert AllocatorTy to an output")
            }
        }
    }

    fn usize(&self) -> P<Ty> {
        let usize = self.cx.path_ident(self.span, Ident::from_str("usize"));
        self.cx.ty_path(usize)
    }

    fn ptr_u8(&self) -> P<Ty> {
        let u8 = self.cx.path_ident(self.span, Ident::from_str("u8"));
        let ty_u8 = self.cx.ty_path(u8);
        self.cx.ty_ptr(self.span, ty_u8, Mutability::Mutable)
    }
}
