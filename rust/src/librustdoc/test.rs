use errors::{self, FatalError};
use errors::emitter::ColorConfig;
use rustc_data_structures::sync::Lrc;
use rustc_lint;
use rustc_driver::{self, driver, target_features, Compilation};
use rustc_driver::driver::phase_2_configure_and_expand;
use rustc_metadata::cstore::CStore;
use rustc_metadata::dynamic_lib::DynamicLibrary;
use rustc_resolve::MakeGlobMap;
use rustc::hir;
use rustc::hir::intravisit;
use rustc::session::{self, CompileIncomplete, config};
use rustc::session::config::{OutputType, OutputTypes, Externs, CodegenOptions};
use rustc::session::search_paths::{SearchPath, PathKind};
use syntax::ast;
use syntax::source_map::SourceMap;
use syntax::edition::Edition;
use syntax::feature_gate::UnstableFeatures;
use syntax::with_globals;
use syntax_pos::{BytePos, DUMMY_SP, Pos, Span, FileName};
use tempfile::Builder as TempFileBuilder;
use testing;

use std::env;
use std::ffi::OsString;
use std::io::prelude::*;
use std::io;
use std::path::PathBuf;
use std::panic::{self, AssertUnwindSafe};
use std::process::Command;
use std::str;
use std::sync::{Arc, Mutex};

use clean::Attributes;
use config::Options;
use html::markdown::{self, ErrorCodes, LangString};

#[derive(Clone, Default)]
pub struct TestOptions {
    /// Whether to disable the default `extern crate my_crate;` when creating doctests.
    pub no_crate_inject: bool,
    /// Whether to emit compilation warnings when compiling doctests. Setting this will suppress
    /// the default `#![allow(unused)]`.
    pub display_warnings: bool,
    /// Additional crate-level attributes to add to doctests.
    pub attrs: Vec<String>,
}

pub fn run(mut options: Options) -> isize {
    let input = config::Input::File(options.input.clone());

    let sessopts = config::Options {
        maybe_sysroot: options.maybe_sysroot.clone().or_else(
            || Some(env::current_exe().unwrap().parent().unwrap().parent().unwrap().to_path_buf())),
        search_paths: options.libs.clone(),
        crate_types: vec![config::CrateType::Dylib],
        cg: options.codegen_options.clone(),
        externs: options.externs.clone(),
        unstable_features: UnstableFeatures::from_environment(),
        lint_cap: Some(::rustc::lint::Level::Allow),
        actually_rustdoc: true,
        debugging_opts: config::DebuggingOptions {
            ..config::basic_debugging_options()
        },
        edition: options.edition,
        ..config::Options::default()
    };
    driver::spawn_thread_pool(sessopts, |sessopts| {
        let source_map = Lrc::new(SourceMap::new(sessopts.file_path_mapping()));
        let handler =
            errors::Handler::with_tty_emitter(ColorConfig::Auto,
                                            true, false,
                                            Some(source_map.clone()));

        let mut sess = session::build_session_(
            sessopts, Some(options.input), handler, source_map.clone(),
        );
        let codegen_backend = rustc_driver::get_codegen_backend(&sess);
        let cstore = CStore::new(codegen_backend.metadata_loader());
        rustc_lint::register_builtins(&mut sess.lint_store.borrow_mut(), Some(&sess));

        let mut cfg = config::build_configuration(&sess,
                                                  config::parse_cfgspecs(options.cfgs.clone()));
        target_features::add_configuration(&mut cfg, &sess, &*codegen_backend);
        sess.parse_sess.config = cfg;

        let krate =
            match driver::phase_1_parse_input(&driver::CompileController::basic(), &sess, &input) {
                Ok(krate) => krate,
                Err(mut e) => {
                    e.emit();
                    FatalError.raise();
                }
            };
        let driver::ExpansionResult { defs, mut hir_forest, .. } = {
            phase_2_configure_and_expand(
                &sess,
                &cstore,
                krate,
                None,
                "rustdoc-test",
                None,
                MakeGlobMap::No,
                |_| Ok(()),
            ).expect("phase_2_configure_and_expand aborted in rustdoc!")
        };

        let crate_name = options.crate_name.unwrap_or_else(|| {
            ::rustc_codegen_utils::link::find_crate_name(None, &hir_forest.krate().attrs, &input)
        });
        let mut opts = scrape_test_config(hir_forest.krate());
        opts.display_warnings |= options.display_warnings;
        let mut collector = Collector::new(
            crate_name,
            options.cfgs,
            options.libs,
            options.codegen_options,
            options.externs,
            false,
            opts,
            options.maybe_sysroot,
            Some(source_map),
            None,
            options.linker,
            options.edition
        );

        {
            let map = hir::map::map_crate(&sess, &cstore, &mut hir_forest, &defs);
            let krate = map.krate();
            let mut hir_collector = HirCollector {
                sess: &sess,
                collector: &mut collector,
                map: &map,
                codes: ErrorCodes::from(sess.opts.unstable_features.is_nightly_build()),
            };
            hir_collector.visit_testable("".to_string(), &krate.attrs, |this| {
                intravisit::walk_crate(this, krate);
            });
        }

        options.test_args.insert(0, "rustdoctest".to_string());

        testing::test_main(&options.test_args,
                        collector.tests.into_iter().collect(),
                        testing::Options::new().display_output(options.display_warnings));
        0
    })
}

// Look for `#![doc(test(no_crate_inject))]`, used by crates in the std facade.
fn scrape_test_config(krate: &::rustc::hir::Crate) -> TestOptions {
    use syntax::print::pprust;

    let mut opts = TestOptions {
        no_crate_inject: false,
        display_warnings: false,
        attrs: Vec::new(),
    };

    let test_attrs: Vec<_> = krate.attrs.iter()
        .filter(|a| a.check_name("doc"))
        .flat_map(|a| a.meta_item_list().unwrap_or_else(Vec::new))
        .filter(|a| a.check_name("test"))
        .collect();
    let attrs = test_attrs.iter().flat_map(|a| a.meta_item_list().unwrap_or(&[]));

    for attr in attrs {
        if attr.check_name("no_crate_inject") {
            opts.no_crate_inject = true;
        }
        if attr.check_name("attr") {
            if let Some(l) = attr.meta_item_list() {
                for item in l {
                    opts.attrs.push(pprust::meta_list_item_to_string(item));
                }
            }
        }
    }

    opts
}

fn run_test(test: &str, cratename: &str, filename: &FileName, line: usize,
            cfgs: Vec<String>, libs: Vec<SearchPath>,
            cg: CodegenOptions, externs: Externs,
            should_panic: bool, no_run: bool, as_test_harness: bool,
            compile_fail: bool, mut error_codes: Vec<String>, opts: &TestOptions,
            maybe_sysroot: Option<PathBuf>, linker: Option<PathBuf>, edition: Edition) {
    // The test harness wants its own `main` and top-level functions, so
    // never wrap the test in `fn main() { ... }`.
    let (test, line_offset) = make_test(test, Some(cratename), as_test_harness, opts);
    // FIXME(#44940): if doctests ever support path remapping, then this filename
    // needs to be the result of `SourceMap::span_to_unmapped_path`.
    let path = match filename {
        FileName::Real(path) => path.clone(),
        _ => PathBuf::from(r"doctest.rs"),
    };

    let input = config::Input::Str {
        name: FileName::DocTest(path, line as isize - line_offset as isize),
        input: test,
    };
    let outputs = OutputTypes::new(&[(OutputType::Exe, None)]);

    let sessopts = config::Options {
        maybe_sysroot: maybe_sysroot.or_else(
            || Some(env::current_exe().unwrap().parent().unwrap().parent().unwrap().to_path_buf())),
        search_paths: libs,
        crate_types: vec![config::CrateType::Executable],
        output_types: outputs,
        externs,
        cg: config::CodegenOptions {
            linker,
            ..cg
        },
        test: as_test_harness,
        unstable_features: UnstableFeatures::from_environment(),
        debugging_opts: config::DebuggingOptions {
            ..config::basic_debugging_options()
        },
        edition,
        ..config::Options::default()
    };

    // Shuffle around a few input and output handles here. We're going to pass
    // an explicit handle into rustc to collect output messages, but we also
    // want to catch the error message that rustc prints when it fails.
    //
    // We take our thread-local stderr (likely set by the test runner) and replace
    // it with a sink that is also passed to rustc itself. When this function
    // returns the output of the sink is copied onto the output of our own thread.
    //
    // The basic idea is to not use a default Handler for rustc, and then also
    // not print things by default to the actual stderr.
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            Write::write(&mut *self.0.lock().unwrap(), data)
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }
    struct Bomb(Arc<Mutex<Vec<u8>>>, Box<dyn Write+Send>);
    impl Drop for Bomb {
        fn drop(&mut self) {
            let _ = self.1.write_all(&self.0.lock().unwrap());
        }
    }
    let data = Arc::new(Mutex::new(Vec::new()));

    let old = io::set_panic(Some(box Sink(data.clone())));
    let _bomb = Bomb(data.clone(), old.unwrap_or(box io::stdout()));

    let (libdir, outdir, compile_result) = driver::spawn_thread_pool(sessopts, |sessopts| {
        let source_map = Lrc::new(SourceMap::new(sessopts.file_path_mapping()));
        let emitter = errors::emitter::EmitterWriter::new(box Sink(data.clone()),
                                                        Some(source_map.clone()),
                                                        false,
                                                        false);

        // Compile the code
        let diagnostic_handler = errors::Handler::with_emitter(true, false, box emitter);

        let mut sess = session::build_session_(
            sessopts, None, diagnostic_handler, source_map,
        );
        let codegen_backend = rustc_driver::get_codegen_backend(&sess);
        let cstore = CStore::new(codegen_backend.metadata_loader());
        rustc_lint::register_builtins(&mut sess.lint_store.borrow_mut(), Some(&sess));

        let outdir = Mutex::new(
            TempFileBuilder::new().prefix("rustdoctest").tempdir().expect("rustdoc needs a tempdir")
        );
        let libdir = sess.target_filesearch(PathKind::All).get_lib_path();
        let mut control = driver::CompileController::basic();

        let mut cfg = config::build_configuration(&sess, config::parse_cfgspecs(cfgs.clone()));
        target_features::add_configuration(&mut cfg, &sess, &*codegen_backend);
        sess.parse_sess.config = cfg;

        let out = Some(outdir.lock().unwrap().path().join("rust_out"));

        if no_run {
            control.after_analysis.stop = Compilation::Stop;
        }

        let res = panic::catch_unwind(AssertUnwindSafe(|| {
            driver::compile_input(
                codegen_backend,
                &sess,
                &cstore,
                &None,
                &input,
                &None,
                &out,
                None,
                &control
            )
        }));

        let compile_result = match res {
            Ok(Ok(())) | Ok(Err(CompileIncomplete::Stopped)) => Ok(()),
            Err(_) | Ok(Err(CompileIncomplete::Errored(_))) => Err(())
        };

        (libdir, outdir, compile_result)
    });

    match (compile_result, compile_fail) {
        (Ok(()), true) => {
            panic!("test compiled while it wasn't supposed to")
        }
        (Ok(()), false) => {}
        (Err(()), true) => {
            if error_codes.len() > 0 {
                let out = String::from_utf8(data.lock().unwrap().to_vec()).unwrap();
                error_codes.retain(|err| !out.contains(err));
            }
        }
        (Err(()), false) => {
            panic!("couldn't compile the test")
        }
    }

    if error_codes.len() > 0 {
        panic!("Some expected error codes were not found: {:?}", error_codes);
    }

    if no_run { return }

    // Run the code!
    //
    // We're careful to prepend the *target* dylib search path to the child's
    // environment to ensure that the target loads the right libraries at
    // runtime. It would be a sad day if the *host* libraries were loaded as a
    // mistake.
    let mut cmd = Command::new(&outdir.lock().unwrap().path().join("rust_out"));
    let var = DynamicLibrary::envvar();
    let newpath = {
        let path = env::var_os(var).unwrap_or(OsString::new());
        let mut path = env::split_paths(&path).collect::<Vec<_>>();
        path.insert(0, libdir);
        env::join_paths(path).unwrap()
    };
    cmd.env(var, &newpath);

    match cmd.output() {
        Err(e) => panic!("couldn't run the test: {}{}", e,
                        if e.kind() == io::ErrorKind::PermissionDenied {
                            " - maybe your tempdir is mounted with noexec?"
                        } else { "" }),
        Ok(out) => {
            if should_panic && out.status.success() {
                panic!("test executable succeeded when it should have failed");
            } else if !should_panic && !out.status.success() {
                panic!("test executable failed:\n{}\n{}\n",
                       str::from_utf8(&out.stdout).unwrap_or(""),
                       str::from_utf8(&out.stderr).unwrap_or(""));
            }
        }
    }
}

/// Makes the test file. Also returns the number of lines before the code begins
pub fn make_test(s: &str,
                 cratename: Option<&str>,
                 dont_insert_main: bool,
                 opts: &TestOptions)
                 -> (String, usize) {
    let (crate_attrs, everything_else, crates) = partition_source(s);
    let everything_else = everything_else.trim();
    let mut line_offset = 0;
    let mut prog = String::new();

    if opts.attrs.is_empty() && !opts.display_warnings {
        // If there aren't any attributes supplied by #![doc(test(attr(...)))], then allow some
        // lints that are commonly triggered in doctests. The crate-level test attributes are
        // commonly used to make tests fail in case they trigger warnings, so having this there in
        // that case may cause some tests to pass when they shouldn't have.
        prog.push_str("#![allow(unused)]\n");
        line_offset += 1;
    }

    // Next, any attributes that came from the crate root via #![doc(test(attr(...)))].
    for attr in &opts.attrs {
        prog.push_str(&format!("#![{}]\n", attr));
        line_offset += 1;
    }

    // Now push any outer attributes from the example, assuming they
    // are intended to be crate attributes.
    prog.push_str(&crate_attrs);
    prog.push_str(&crates);

    // Uses libsyntax to parse the doctest and find if there's a main fn and the extern
    // crate already is included.
    let (already_has_main, already_has_extern_crate) = crate::syntax::with_globals(|| {
        use crate::syntax::{ast, parse::{self, ParseSess}, source_map::FilePathMapping};
        use crate::syntax_pos::FileName;
        use errors::emitter::EmitterWriter;
        use errors::Handler;

        let filename = FileName::anon_source_code(s);
        let source = crates + &everything_else;

        // Any errors in parsing should also appear when the doctest is compiled for real, so just
        // send all the errors that libsyntax emits directly into a `Sink` instead of stderr.
        let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
        let emitter = EmitterWriter::new(box io::sink(), None, false, false);
        let handler = Handler::with_emitter(false, false, box emitter);
        let sess = ParseSess::with_span_handler(handler, cm);

        let mut found_main = false;
        let mut found_extern_crate = cratename.is_none();

        let mut parser = match parse::maybe_new_parser_from_source_str(&sess, filename, source) {
            Ok(p) => p,
            Err(errs) => {
                for mut err in errs {
                    err.cancel();
                }

                return (found_main, found_extern_crate);
            }
        };

        loop {
            match parser.parse_item() {
                Ok(Some(item)) => {
                    if !found_main {
                        if let ast::ItemKind::Fn(..) = item.node {
                            if item.ident.as_str() == "main" {
                                found_main = true;
                            }
                        }
                    }

                    if !found_extern_crate {
                        if let ast::ItemKind::ExternCrate(original) = item.node {
                            // This code will never be reached if `cratename` is none because
                            // `found_extern_crate` is initialized to `true` if it is none.
                            let cratename = cratename.unwrap();

                            match original {
                                Some(name) => found_extern_crate = name.as_str() == cratename,
                                None => found_extern_crate = item.ident.as_str() == cratename,
                            }
                        }
                    }

                    if found_main && found_extern_crate {
                        break;
                    }
                }
                Ok(None) => break,
                Err(mut e) => {
                    e.cancel();
                    break;
                }
            }
        }

        (found_main, found_extern_crate)
    });

    // Don't inject `extern crate std` because it's already injected by the
    // compiler.
    if !already_has_extern_crate && !opts.no_crate_inject && cratename != Some("std") {
        if let Some(cratename) = cratename {
            // Make sure its actually used if not included.
            if s.contains(cratename) {
                prog.push_str(&format!("extern crate {};\n", cratename));
                line_offset += 1;
            }
        }
    }

    if dont_insert_main || already_has_main {
        prog.push_str(everything_else);
    } else {
        prog.push_str("fn main() {\n");
        line_offset += 1;
        prog.push_str(everything_else);
        prog.push_str("\n}");
    }

    debug!("final doctest:\n{}", prog);

    (prog, line_offset)
}

// FIXME(aburka): use a real parser to deal with multiline attributes
fn partition_source(s: &str) -> (String, String, String) {
    #[derive(Copy, Clone, PartialEq)]
    enum PartitionState {
        Attrs,
        Crates,
        Other,
    }
    let mut state = PartitionState::Attrs;
    let mut before = String::new();
    let mut crates = String::new();
    let mut after = String::new();

    for line in s.lines() {
        let trimline = line.trim();

        // FIXME(misdreavus): if a doc comment is placed on an extern crate statement, it will be
        // shunted into "everything else"
        match state {
            PartitionState::Attrs => {
                state = if trimline.starts_with("#![") ||
                    trimline.chars().all(|c| c.is_whitespace()) ||
                    (trimline.starts_with("//") && !trimline.starts_with("///"))
                {
                    PartitionState::Attrs
                } else if trimline.starts_with("extern crate") ||
                    trimline.starts_with("#[macro_use] extern crate")
                {
                    PartitionState::Crates
                } else {
                    PartitionState::Other
                };
            }
            PartitionState::Crates => {
                state = if trimline.starts_with("extern crate") ||
                    trimline.starts_with("#[macro_use] extern crate") ||
                    trimline.chars().all(|c| c.is_whitespace()) ||
                    (trimline.starts_with("//") && !trimline.starts_with("///"))
                {
                    PartitionState::Crates
                } else {
                    PartitionState::Other
                };
            }
            PartitionState::Other => {}
        }

        match state {
            PartitionState::Attrs => {
                before.push_str(line);
                before.push_str("\n");
            }
            PartitionState::Crates => {
                crates.push_str(line);
                crates.push_str("\n");
            }
            PartitionState::Other => {
                after.push_str(line);
                after.push_str("\n");
            }
        }
    }

    debug!("before:\n{}", before);
    debug!("crates:\n{}", crates);
    debug!("after:\n{}", after);

    (before, after, crates)
}

pub trait Tester {
    fn add_test(&mut self, test: String, config: LangString, line: usize);
    fn get_line(&self) -> usize {
        0
    }
    fn register_header(&mut self, _name: &str, _level: u32) {}
}

pub struct Collector {
    pub tests: Vec<testing::TestDescAndFn>,

    // The name of the test displayed to the user, separated by `::`.
    //
    // In tests from Rust source, this is the path to the item
    // e.g., `["std", "vec", "Vec", "push"]`.
    //
    // In tests from a markdown file, this is the titles of all headers (h1~h6)
    // of the sections that contain the code block, e.g., if the markdown file is
    // written as:
    //
    // ``````markdown
    // # Title
    //
    // ## Subtitle
    //
    // ```rust
    // assert!(true);
    // ```
    // ``````
    //
    // the `names` vector of that test will be `["Title", "Subtitle"]`.
    names: Vec<String>,

    cfgs: Vec<String>,
    libs: Vec<SearchPath>,
    cg: CodegenOptions,
    externs: Externs,
    use_headers: bool,
    cratename: String,
    opts: TestOptions,
    maybe_sysroot: Option<PathBuf>,
    position: Span,
    source_map: Option<Lrc<SourceMap>>,
    filename: Option<PathBuf>,
    linker: Option<PathBuf>,
    edition: Edition,
}

impl Collector {
    pub fn new(cratename: String, cfgs: Vec<String>, libs: Vec<SearchPath>, cg: CodegenOptions,
               externs: Externs, use_headers: bool, opts: TestOptions,
               maybe_sysroot: Option<PathBuf>, source_map: Option<Lrc<SourceMap>>,
               filename: Option<PathBuf>, linker: Option<PathBuf>, edition: Edition) -> Collector {
        Collector {
            tests: Vec::new(),
            names: Vec::new(),
            cfgs,
            libs,
            cg,
            externs,
            use_headers,
            cratename,
            opts,
            maybe_sysroot,
            position: DUMMY_SP,
            source_map,
            filename,
            linker,
            edition,
        }
    }

    fn generate_name(&self, line: usize, filename: &FileName) -> String {
        format!("{} - {} (line {})", filename, self.names.join("::"), line)
    }

    pub fn set_position(&mut self, position: Span) {
        self.position = position;
    }

    fn get_filename(&self) -> FileName {
        if let Some(ref source_map) = self.source_map {
            let filename = source_map.span_to_filename(self.position);
            if let FileName::Real(ref filename) = filename {
                if let Ok(cur_dir) = env::current_dir() {
                    if let Ok(path) = filename.strip_prefix(&cur_dir) {
                        return path.to_owned().into();
                    }
                }
            }
            filename
        } else if let Some(ref filename) = self.filename {
            filename.clone().into()
        } else {
            FileName::Custom("input".to_owned())
        }
    }
}

impl Tester for Collector {
    fn add_test(&mut self, test: String, config: LangString, line: usize) {
        let filename = self.get_filename();
        let name = self.generate_name(line, &filename);
        let cfgs = self.cfgs.clone();
        let libs = self.libs.clone();
        let cg = self.cg.clone();
        let externs = self.externs.clone();
        let cratename = self.cratename.to_string();
        let opts = self.opts.clone();
        let maybe_sysroot = self.maybe_sysroot.clone();
        let linker = self.linker.clone();
        let edition = config.edition.unwrap_or(self.edition);
        debug!("Creating test {}: {}", name, test);
        self.tests.push(testing::TestDescAndFn {
            desc: testing::TestDesc {
                name: testing::DynTestName(name.clone()),
                ignore: config.ignore,
                // compiler failures are test failures
                should_panic: testing::ShouldPanic::No,
                allow_fail: config.allow_fail,
            },
            testfn: testing::DynTestFn(box move || {
                let panic = io::set_panic(None);
                let print = io::set_print(None);
                match {
                    rustc_driver::in_named_rustc_thread(name, move || with_globals(move || {
                        io::set_panic(panic);
                        io::set_print(print);
                        run_test(&test,
                                 &cratename,
                                 &filename,
                                 line,
                                 cfgs,
                                 libs,
                                 cg,
                                 externs,
                                 config.should_panic,
                                 config.no_run,
                                 config.test_harness,
                                 config.compile_fail,
                                 config.error_codes,
                                 &opts,
                                 maybe_sysroot,
                                 linker,
                                 edition)
                    }))
                } {
                    Ok(()) => (),
                    Err(err) => panic::resume_unwind(err),
                }
            }),
        });
    }

    fn get_line(&self) -> usize {
        if let Some(ref source_map) = self.source_map {
            let line = self.position.lo().to_usize();
            let line = source_map.lookup_char_pos(BytePos(line as u32)).line;
            if line > 0 { line - 1 } else { line }
        } else {
            0
        }
    }

    fn register_header(&mut self, name: &str, level: u32) {
        if self.use_headers {
            // We use these headings as test names, so it's good if
            // they're valid identifiers.
            let name = name.chars().enumerate().map(|(i, c)| {
                    if (i == 0 && c.is_xid_start()) ||
                        (i != 0 && c.is_xid_continue()) {
                        c
                    } else {
                        '_'
                    }
                }).collect::<String>();

            // Here we try to efficiently assemble the header titles into the
            // test name in the form of `h1::h2::h3::h4::h5::h6`.
            //
            // Suppose that originally `self.names` contains `[h1, h2, h3]`...
            let level = level as usize;
            if level <= self.names.len() {
                // ... Consider `level == 2`. All headers in the lower levels
                // are irrelevant in this new level. So we should reset
                // `self.names` to contain headers until <h2>, and replace that
                // slot with the new name: `[h1, name]`.
                self.names.truncate(level);
                self.names[level - 1] = name;
            } else {
                // ... On the other hand, consider `level == 5`. This means we
                // need to extend `self.names` to contain five headers. We fill
                // in the missing level (<h4>) with `_`. Thus `self.names` will
                // become `[h1, h2, h3, "_", name]`.
                if level - 1 > self.names.len() {
                    self.names.resize(level - 1, "_".to_owned());
                }
                self.names.push(name);
            }
        }
    }
}

struct HirCollector<'a, 'hir: 'a> {
    sess: &'a session::Session,
    collector: &'a mut Collector,
    map: &'a hir::map::Map<'hir>,
    codes: ErrorCodes,
}

impl<'a, 'hir> HirCollector<'a, 'hir> {
    fn visit_testable<F: FnOnce(&mut Self)>(&mut self,
                                            name: String,
                                            attrs: &[ast::Attribute],
                                            nested: F) {
        let mut attrs = Attributes::from_ast(self.sess.diagnostic(), attrs);
        if let Some(ref cfg) = attrs.cfg {
            if !cfg.matches(&self.sess.parse_sess, Some(&self.sess.features_untracked())) {
                return;
            }
        }

        let has_name = !name.is_empty();
        if has_name {
            self.collector.names.push(name);
        }

        attrs.collapse_doc_comments();
        attrs.unindent_doc_comments();
        // The collapse-docs pass won't combine sugared/raw doc attributes, or included files with
        // anything else, this will combine them for us.
        if let Some(doc) = attrs.collapsed_doc_value() {
            self.collector.set_position(attrs.span.unwrap_or(DUMMY_SP));
            let res = markdown::find_testable_code(&doc, self.collector, self.codes);
            if let Err(err) = res {
                self.sess.diagnostic().span_warn(attrs.span.unwrap_or(DUMMY_SP),
                    &err.to_string());
            }
        }

        nested(self);

        if has_name {
            self.collector.names.pop();
        }
    }
}

impl<'a, 'hir> intravisit::Visitor<'hir> for HirCollector<'a, 'hir> {
    fn nested_visit_map<'this>(&'this mut self) -> intravisit::NestedVisitorMap<'this, 'hir> {
        intravisit::NestedVisitorMap::All(&self.map)
    }

    fn visit_item(&mut self, item: &'hir hir::Item) {
        let name = if let hir::ItemKind::Impl(.., ref ty, _) = item.node {
            self.map.node_to_pretty_string(ty.id)
        } else {
            item.ident.to_string()
        };

        self.visit_testable(name, &item.attrs, |this| {
            intravisit::walk_item(this, item);
        });
    }

    fn visit_trait_item(&mut self, item: &'hir hir::TraitItem) {
        self.visit_testable(item.ident.to_string(), &item.attrs, |this| {
            intravisit::walk_trait_item(this, item);
        });
    }

    fn visit_impl_item(&mut self, item: &'hir hir::ImplItem) {
        self.visit_testable(item.ident.to_string(), &item.attrs, |this| {
            intravisit::walk_impl_item(this, item);
        });
    }

    fn visit_foreign_item(&mut self, item: &'hir hir::ForeignItem) {
        self.visit_testable(item.ident.to_string(), &item.attrs, |this| {
            intravisit::walk_foreign_item(this, item);
        });
    }

    fn visit_variant(&mut self,
                     v: &'hir hir::Variant,
                     g: &'hir hir::Generics,
                     item_id: ast::NodeId) {
        self.visit_testable(v.node.ident.to_string(), &v.node.attrs, |this| {
            intravisit::walk_variant(this, v, g, item_id);
        });
    }

    fn visit_struct_field(&mut self, f: &'hir hir::StructField) {
        self.visit_testable(f.ident.to_string(), &f.attrs, |this| {
            intravisit::walk_struct_field(this, f);
        });
    }

    fn visit_macro_def(&mut self, macro_def: &'hir hir::MacroDef) {
        self.visit_testable(macro_def.name.to_string(), &macro_def.attrs, |_| ());
    }
}

#[cfg(test)]
mod tests {
    use super::{TestOptions, make_test};

    #[test]
    fn make_test_basic() {
        //basic use: wraps with `fn main`, adds `#![allow(unused)]`
        let opts = TestOptions::default();
        let input =
"assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
fn main() {
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, None, false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_crate_name_no_use() {
        // If you give a crate name but *don't* use it within the test, it won't bother inserting
        // the `extern crate` statement.
        let opts = TestOptions::default();
        let input =
"assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
fn main() {
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_crate_name() {
        // If you give a crate name and use it within the test, it will insert an `extern crate`
        // statement before `fn main`.
        let opts = TestOptions::default();
        let input =
"use asdf::qwop;
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
extern crate asdf;
fn main() {
use asdf::qwop;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 3));
    }

    #[test]
    fn make_test_no_crate_inject() {
        // Even if you do use the crate within the test, setting `opts.no_crate_inject` will skip
        // adding it anyway.
        let opts = TestOptions {
            no_crate_inject: true,
            display_warnings: false,
            attrs: vec![],
        };
        let input =
"use asdf::qwop;
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
fn main() {
use asdf::qwop;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_ignore_std() {
        // Even if you include a crate name, and use it in the doctest, we still won't include an
        // `extern crate` statement if the crate is "std" -- that's included already by the
        // compiler!
        let opts = TestOptions::default();
        let input =
"use std::*;
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
fn main() {
use std::*;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("std"), false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_manual_extern_crate() {
        // When you manually include an `extern crate` statement in your doctest, `make_test`
        // assumes you've included one for your own crate too.
        let opts = TestOptions::default();
        let input =
"extern crate asdf;
use asdf::qwop;
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
extern crate asdf;
fn main() {
use asdf::qwop;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_manual_extern_crate_with_macro_use() {
        let opts = TestOptions::default();
        let input =
"#[macro_use] extern crate asdf;
use asdf::qwop;
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
#[macro_use] extern crate asdf;
fn main() {
use asdf::qwop;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_opts_attrs() {
        // If you supplied some doctest attributes with `#![doc(test(attr(...)))]`, it will use
        // those instead of the stock `#![allow(unused)]`.
        let mut opts = TestOptions::default();
        opts.attrs.push("feature(sick_rad)".to_string());
        let input =
"use asdf::qwop;
assert_eq!(2+2, 4);";
        let expected =
"#![feature(sick_rad)]
extern crate asdf;
fn main() {
use asdf::qwop;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 3));

        // Adding more will also bump the returned line offset.
        opts.attrs.push("feature(hella_dope)".to_string());
        let expected =
"#![feature(sick_rad)]
#![feature(hella_dope)]
extern crate asdf;
fn main() {
use asdf::qwop;
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 4));
    }

    #[test]
    fn make_test_crate_attrs() {
        // Including inner attributes in your doctest will apply them to the whole "crate", pasting
        // them outside the generated main function.
        let opts = TestOptions::default();
        let input =
"#![feature(sick_rad)]
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
#![feature(sick_rad)]
fn main() {
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, None, false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_with_main() {
        // Including your own `fn main` wrapper lets the test use it verbatim.
        let opts = TestOptions::default();
        let input =
"fn main() {
    assert_eq!(2+2, 4);
}";
        let expected =
"#![allow(unused)]
fn main() {
    assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, None, false, &opts);
        assert_eq!(output, (expected, 1));
    }

    #[test]
    fn make_test_fake_main() {
        // ... but putting it in a comment will still provide a wrapper.
        let opts = TestOptions::default();
        let input =
"//Ceci n'est pas une `fn main`
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
//Ceci n'est pas une `fn main`
fn main() {
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, None, false, &opts);
        assert_eq!(output, (expected, 2));
    }

    #[test]
    fn make_test_dont_insert_main() {
        // Even with that, if you set `dont_insert_main`, it won't create the `fn main` wrapper.
        let opts = TestOptions::default();
        let input =
"//Ceci n'est pas une `fn main`
assert_eq!(2+2, 4);";
        let expected =
"#![allow(unused)]
//Ceci n'est pas une `fn main`
assert_eq!(2+2, 4);".to_string();
        let output = make_test(input, None, true, &opts);
        assert_eq!(output, (expected, 1));
    }

    #[test]
    fn make_test_display_warnings() {
        // If the user is asking to display doctest warnings, suppress the default `allow(unused)`.
        let mut opts = TestOptions::default();
        opts.display_warnings = true;
        let input =
"assert_eq!(2+2, 4);";
        let expected =
"fn main() {
assert_eq!(2+2, 4);
}".to_string();
        let output = make_test(input, None, false, &opts);
        assert_eq!(output, (expected, 1));
    }

    #[test]
    fn make_test_issues_21299_33731() {
        let opts = TestOptions::default();

        let input =
"// fn main
assert_eq!(2+2, 4);";

        let expected =
"#![allow(unused)]
// fn main
fn main() {
assert_eq!(2+2, 4);
}".to_string();

        let output = make_test(input, None, false, &opts);
        assert_eq!(output, (expected, 2));

        let input =
"extern crate hella_qwop;
assert_eq!(asdf::foo, 4);";

        let expected =
"#![allow(unused)]
extern crate hella_qwop;
extern crate asdf;
fn main() {
assert_eq!(asdf::foo, 4);
}".to_string();

        let output = make_test(input, Some("asdf"), false, &opts);
        assert_eq!(output, (expected, 3));
    }
}
