// Code that generates a test runner to run all the tests in a crate

#![allow(dead_code)]
#![allow(unused_imports)]

use self::HasTestSignature::*;

use std::iter;
use std::slice;
use std::mem;
use std::vec;
use attr::{self, HasAttrs};
use syntax_pos::{self, DUMMY_SP, NO_EXPANSION, Span, SourceFile, BytePos};

use source_map::{self, SourceMap, ExpnInfo, MacroAttribute, dummy_spanned, respan};
use errors;
use config;
use entry::{self, EntryPointType};
use ext::base::{ExtCtxt, Resolver};
use ext::build::AstBuilder;
use ext::expand::ExpansionConfig;
use ext::hygiene::{self, Mark, SyntaxContext};
use fold::Folder;
use feature_gate::Features;
use util::move_map::MoveMap;
use fold::{self, ExpectOne};
use parse::{token, ParseSess};
use print::pprust;
use ast::{self, Ident};
use ptr::P;
use smallvec::SmallVec;
use symbol::{self, Symbol, keywords};
use ThinVec;

struct Test {
    span: Span,
    path: Vec<Ident>,
}

struct TestCtxt<'a> {
    span_diagnostic: &'a errors::Handler,
    path: Vec<Ident>,
    ext_cx: ExtCtxt<'a>,
    test_cases: Vec<Test>,
    reexport_test_harness_main: Option<Symbol>,
    is_libtest: bool,
    ctxt: SyntaxContext,
    features: &'a Features,
    test_runner: Option<ast::Path>,

    // top-level re-export submodule, filled out after folding is finished
    toplevel_reexport: Option<Ident>,
}

// Traverse the crate, collecting all the test functions, eliding any
// existing main functions, and synthesizing a main test harness
pub fn modify_for_testing(sess: &ParseSess,
                          resolver: &mut dyn Resolver,
                          should_test: bool,
                          krate: ast::Crate,
                          span_diagnostic: &errors::Handler,
                          features: &Features) -> ast::Crate {
    // Check for #[reexport_test_harness_main = "some_name"] which
    // creates a `use __test::main as some_name;`. This needs to be
    // unconditional, so that the attribute is still marked as used in
    // non-test builds.
    let reexport_test_harness_main =
        attr::first_attr_value_str_by_name(&krate.attrs,
                                           "reexport_test_harness_main");

    // Do this here so that the test_runner crate attribute gets marked as used
    // even in non-test builds
    let test_runner = get_test_runner(span_diagnostic, &krate);

    if should_test {
        generate_test_harness(sess, resolver, reexport_test_harness_main,
                              krate, span_diagnostic, features, test_runner)
    } else {
        krate
    }
}

struct TestHarnessGenerator<'a> {
    cx: TestCtxt<'a>,
    tests: Vec<Ident>,

    // submodule name, gensym'd identifier for re-exports
    tested_submods: Vec<(Ident, Ident)>,
}

impl<'a> fold::Folder for TestHarnessGenerator<'a> {
    fn fold_crate(&mut self, c: ast::Crate) -> ast::Crate {
        let mut folded = fold::noop_fold_crate(c, self);

        // Create a main function to run our tests
        let test_main = {
            let unresolved = mk_main(&mut self.cx);
            self.cx.ext_cx.monotonic_expander().fold_item(unresolved).pop().unwrap()
        };

        folded.module.items.push(test_main);
        folded
    }

    fn fold_item(&mut self, i: P<ast::Item>) -> SmallVec<[P<ast::Item>; 1]> {
        let ident = i.ident;
        if ident.name != keywords::Invalid.name() {
            self.cx.path.push(ident);
        }
        debug!("current path: {}", path_name_i(&self.cx.path));

        let mut item = i.into_inner();
        if is_test_case(&item) {
            debug!("this is a test item");

            let test = Test {
                span: item.span,
                path: self.cx.path.clone(),
            };
            self.cx.test_cases.push(test);
            self.tests.push(item.ident);
        }

        // We don't want to recurse into anything other than mods, since
        // mods or tests inside of functions will break things
        if let ast::ItemKind::Mod(module) = item.node {
            let tests = mem::replace(&mut self.tests, Vec::new());
            let tested_submods = mem::replace(&mut self.tested_submods, Vec::new());
            let mut mod_folded = fold::noop_fold_mod(module, self);
            let tests = mem::replace(&mut self.tests, tests);
            let tested_submods = mem::replace(&mut self.tested_submods, tested_submods);

            if !tests.is_empty() || !tested_submods.is_empty() {
                let (it, sym) = mk_reexport_mod(&mut self.cx, item.id, tests, tested_submods);
                mod_folded.items.push(it);

                if !self.cx.path.is_empty() {
                    self.tested_submods.push((self.cx.path[self.cx.path.len()-1], sym));
                } else {
                    debug!("pushing nothing, sym: {:?}", sym);
                    self.cx.toplevel_reexport = Some(sym);
                }
            }
            item.node = ast::ItemKind::Mod(mod_folded);
        }
        if ident.name != keywords::Invalid.name() {
            self.cx.path.pop();
        }
        smallvec![P(item)]
    }

    fn fold_mac(&mut self, mac: ast::Mac) -> ast::Mac { mac }
}

/// A folder used to remove any entry points (like fn main) because the harness
/// generator will provide its own
struct EntryPointCleaner {
    // Current depth in the ast
    depth: usize,
}

impl fold::Folder for EntryPointCleaner {
    fn fold_item(&mut self, i: P<ast::Item>) -> SmallVec<[P<ast::Item>; 1]> {
        self.depth += 1;
        let folded = fold::noop_fold_item(i, self).expect_one("noop did something");
        self.depth -= 1;

        // Remove any #[main] or #[start] from the AST so it doesn't
        // clash with the one we're going to add, but mark it as
        // #[allow(dead_code)] to avoid printing warnings.
        let folded = match entry::entry_point_type(&folded, self.depth) {
            EntryPointType::MainNamed |
            EntryPointType::MainAttr |
            EntryPointType::Start =>
                folded.map(|ast::Item {id, ident, attrs, node, vis, span, tokens}| {
                    let allow_ident = Ident::from_str("allow");
                    let dc_nested = attr::mk_nested_word_item(Ident::from_str("dead_code"));
                    let allow_dead_code_item = attr::mk_list_item(DUMMY_SP, allow_ident,
                                                                  vec![dc_nested]);
                    let allow_dead_code = attr::mk_attr_outer(DUMMY_SP,
                                                              attr::mk_attr_id(),
                                                              allow_dead_code_item);

                    ast::Item {
                        id,
                        ident,
                        attrs: attrs.into_iter()
                            .filter(|attr| {
                                !attr.check_name("main") && !attr.check_name("start")
                            })
                            .chain(iter::once(allow_dead_code))
                            .collect(),
                        node,
                        vis,
                        span,
                        tokens,
                    }
                }),
            EntryPointType::None |
            EntryPointType::OtherMain => folded,
        };

        smallvec![folded]
    }

    fn fold_mac(&mut self, mac: ast::Mac) -> ast::Mac { mac }
}

/// Creates an item (specifically a module) that "pub use"s the tests passed in.
/// Each tested submodule will contain a similar reexport module that we will export
/// under the name of the original module. That is, `submod::__test_reexports` is
/// reexported like so `pub use submod::__test_reexports as submod`.
fn mk_reexport_mod(cx: &mut TestCtxt,
                   parent: ast::NodeId,
                   tests: Vec<Ident>,
                   tested_submods: Vec<(Ident, Ident)>)
                   -> (P<ast::Item>, Ident) {
    let super_ = Ident::from_str("super");

    let items = tests.into_iter().map(|r| {
        cx.ext_cx.item_use_simple(DUMMY_SP, dummy_spanned(ast::VisibilityKind::Public),
                                  cx.ext_cx.path(DUMMY_SP, vec![super_, r]))
    }).chain(tested_submods.into_iter().map(|(r, sym)| {
        let path = cx.ext_cx.path(DUMMY_SP, vec![super_, r, sym]);
        cx.ext_cx.item_use_simple_(DUMMY_SP, dummy_spanned(ast::VisibilityKind::Public),
                                   Some(r), path)
    })).collect();

    let reexport_mod = ast::Mod {
        inline: true,
        inner: DUMMY_SP,
        items,
    };

    let sym = Ident::with_empty_ctxt(Symbol::gensym("__test_reexports"));
    let parent = if parent == ast::DUMMY_NODE_ID { ast::CRATE_NODE_ID } else { parent };
    cx.ext_cx.current_expansion.mark = cx.ext_cx.resolver.get_module_scope(parent);
    let it = cx.ext_cx.monotonic_expander().fold_item(P(ast::Item {
        ident: sym,
        attrs: Vec::new(),
        id: ast::DUMMY_NODE_ID,
        node: ast::ItemKind::Mod(reexport_mod),
        vis: dummy_spanned(ast::VisibilityKind::Public),
        span: DUMMY_SP,
        tokens: None,
    })).pop().unwrap();

    (it, sym)
}

/// Crawl over the crate, inserting test reexports and the test main function
fn generate_test_harness(sess: &ParseSess,
                         resolver: &mut dyn Resolver,
                         reexport_test_harness_main: Option<Symbol>,
                         krate: ast::Crate,
                         sd: &errors::Handler,
                         features: &Features,
                         test_runner: Option<ast::Path>) -> ast::Crate {
    // Remove the entry points
    let mut cleaner = EntryPointCleaner { depth: 0 };
    let krate = cleaner.fold_crate(krate);

    let mark = Mark::fresh(Mark::root());

    let mut econfig = ExpansionConfig::default("test".to_string());
    econfig.features = Some(features);

    let cx = TestCtxt {
        span_diagnostic: sd,
        ext_cx: ExtCtxt::new(sess, econfig, resolver),
        path: Vec::new(),
        test_cases: Vec::new(),
        reexport_test_harness_main,
        // N.B., doesn't consider the value of `--crate-name` passed on the command line.
        is_libtest: attr::find_crate_name(&krate.attrs).map(|s| s == "test").unwrap_or(false),
        toplevel_reexport: None,
        ctxt: SyntaxContext::empty().apply_mark(mark),
        features,
        test_runner
    };

    mark.set_expn_info(ExpnInfo {
        call_site: DUMMY_SP,
        def_site: None,
        format: MacroAttribute(Symbol::intern("test_case")),
        allow_internal_unstable: true,
        allow_internal_unsafe: false,
        local_inner_macros: false,
        edition: hygiene::default_edition(),
    });

    TestHarnessGenerator {
        cx,
        tests: Vec::new(),
        tested_submods: Vec::new(),
    }.fold_crate(krate)
}

/// Craft a span that will be ignored by the stability lint's
/// call to source_map's `is_internal` check.
/// The expanded code calls some unstable functions in the test crate.
fn ignored_span(cx: &TestCtxt, sp: Span) -> Span {
    sp.with_ctxt(cx.ctxt)
}

enum HasTestSignature {
    Yes,
    No(BadTestSignature),
}

#[derive(PartialEq)]
enum BadTestSignature {
    NotEvenAFunction,
    WrongTypeSignature,
    NoArgumentsAllowed,
    ShouldPanicOnlyWithNoArgs,
}

/// Creates a function item for use as the main function of a test build.
/// This function will call the `test_runner` as specified by the crate attribute
fn mk_main(cx: &mut TestCtxt) -> P<ast::Item> {
    // Writing this out by hand with 'ignored_span':
    //        pub fn main() {
    //            #![main]
    //            test::test_main_static(::std::os::args().as_slice(), &[..tests]);
    //        }
    let sp = ignored_span(cx, DUMMY_SP);
    let ecx = &cx.ext_cx;
    let test_id = ecx.ident_of("test").gensym();

    // test::test_main_static(...)
    let mut test_runner = cx.test_runner.clone().unwrap_or(
        ecx.path(sp, vec![
            test_id, ecx.ident_of("test_main_static")
        ]));

    test_runner.span = sp;

    let test_main_path_expr = ecx.expr_path(test_runner);
    let call_test_main = ecx.expr_call(sp, test_main_path_expr,
                                       vec![mk_tests_slice(cx)]);
    let call_test_main = ecx.stmt_expr(call_test_main);

    // #![main]
    let main_meta = ecx.meta_word(sp, Symbol::intern("main"));
    let main_attr = ecx.attribute(sp, main_meta);

    // extern crate test as test_gensym
    let test_extern_stmt = ecx.stmt_item(sp, ecx.item(sp,
        test_id,
        vec![],
        ast::ItemKind::ExternCrate(Some(Symbol::intern("test")))
    ));

    // pub fn main() { ... }
    let main_ret_ty = ecx.ty(sp, ast::TyKind::Tup(vec![]));

    // If no test runner is provided we need to import the test crate
    let main_body = if cx.test_runner.is_none() {
        ecx.block(sp, vec![test_extern_stmt, call_test_main])
    } else {
        ecx.block(sp, vec![call_test_main])
    };

    let main = ast::ItemKind::Fn(ecx.fn_decl(vec![], ast::FunctionRetTy::Ty(main_ret_ty)),
                           ast::FnHeader::default(),
                           ast::Generics::default(),
                           main_body);

    // Honor the reexport_test_harness_main attribute
    let main_id = Ident::new(
        cx.reexport_test_harness_main.unwrap_or(Symbol::gensym("main")),
        sp);

    P(ast::Item {
        ident: main_id,
        attrs: vec![main_attr],
        id: ast::DUMMY_NODE_ID,
        node: main,
        vis: dummy_spanned(ast::VisibilityKind::Public),
        span: sp,
        tokens: None,
    })

}

fn path_name_i(idents: &[Ident]) -> String {
    let mut path_name = "".to_string();
    let mut idents_iter = idents.iter().peekable();
    while let Some(ident) = idents_iter.next() {
        path_name.push_str(&ident.as_str());
        if idents_iter.peek().is_some() {
            path_name.push_str("::")
        }
    }
    path_name
}

/// Creates a slice containing every test like so:
/// &[path::to::test1, path::to::test2]
fn mk_tests_slice(cx: &TestCtxt) -> P<ast::Expr> {
    debug!("building test vector from {} tests", cx.test_cases.len());
    let ref ecx = cx.ext_cx;

    ecx.expr_vec_slice(DUMMY_SP,
        cx.test_cases.iter().map(|test| {
            ecx.expr_addr_of(test.span,
                ecx.expr_path(ecx.path(test.span, visible_path(cx, &test.path))))
        }).collect())
}

/// Creates a path from the top-level __test module to the test via __test_reexports
fn visible_path(cx: &TestCtxt, path: &[Ident]) -> Vec<Ident>{
    let mut visible_path = vec![];
    match cx.toplevel_reexport {
        Some(id) => visible_path.push(id),
        None => {
            cx.span_diagnostic.bug("expected to find top-level re-export name, but found None");
        }
    }
    visible_path.extend_from_slice(path);
    visible_path
}

fn is_test_case(i: &ast::Item) -> bool {
    attr::contains_name(&i.attrs, "rustc_test_marker")
}

fn get_test_runner(sd: &errors::Handler, krate: &ast::Crate) -> Option<ast::Path> {
    let test_attr = attr::find_by_name(&krate.attrs, "test_runner")?;
    if let Some(meta_list) = test_attr.meta_item_list() {
        if meta_list.len() != 1 {
            sd.span_fatal(test_attr.span(),
                "#![test_runner(..)] accepts exactly 1 argument").raise()
        }
        Some(meta_list[0].word().as_ref().unwrap().ident.clone())
    } else {
        sd.span_fatal(test_attr.span(),
            "test_runner must be of the form #[test_runner(..)]").raise()
    }
}
