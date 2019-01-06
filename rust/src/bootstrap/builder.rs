use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::fs;
use std::hash::Hash;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::cache::{Cache, Interned, INTERNER};
use crate::check;
use crate::compile;
use crate::dist;
use crate::doc;
use crate::flags::Subcommand;
use crate::install;
use crate::native;
use crate::test;
use crate::tool;
use crate::util::{add_lib_path, exe, libdir};
use crate::{Build, DocTests, Mode, GitRepo};

pub use crate::Compiler;

use petgraph::graph::NodeIndex;
use petgraph::Graph;

pub struct Builder<'a> {
    pub build: &'a Build,
    pub top_stage: u32,
    pub kind: Kind,
    cache: Cache,
    stack: RefCell<Vec<Box<dyn Any>>>,
    time_spent_on_dependencies: Cell<Duration>,
    pub paths: Vec<PathBuf>,
    graph_nodes: RefCell<HashMap<String, NodeIndex>>,
    graph: RefCell<Graph<String, bool>>,
    parent: Cell<Option<NodeIndex>>,
}

impl<'a> Deref for Builder<'a> {
    type Target = Build;

    fn deref(&self) -> &Self::Target {
        self.build
    }
}

pub trait Step: 'static + Clone + Debug + PartialEq + Eq + Hash {
    /// `PathBuf` when directories are created or to return a `Compiler` once
    /// it's been assembled.
    type Output: Clone;

    const DEFAULT: bool = false;

    /// Run this rule for all hosts without cross compiling.
    const ONLY_HOSTS: bool = false;

    /// Primary function to execute this rule. Can call `builder.ensure(...)`
    /// with other steps to run those.
    fn run(self, builder: &Builder) -> Self::Output;

    /// When bootstrap is passed a set of paths, this controls whether this rule
    /// will execute. However, it does not get called in a "default" context
    /// when we are not passed any paths; in that case, make_run is called
    /// directly.
    fn should_run(run: ShouldRun) -> ShouldRun;

    /// Build up a "root" rule, either as a default rule or from a path passed
    /// to us.
    ///
    /// When path is `None`, we are executing in a context where no paths were
    /// passed. When `./x.py build` is run, for example, this rule could get
    /// called if it is in the correct list below with a path of `None`.
    fn make_run(_run: RunConfig) {
        // It is reasonable to not have an implementation of make_run for rules
        // who do not want to get called from the root context. This means that
        // they are likely dependencies (e.g., sysroot creation) or similar, and
        // as such calling them from ./x.py isn't logical.
        unimplemented!()
    }
}

pub struct RunConfig<'a> {
    pub builder: &'a Builder<'a>,
    pub host: Interned<String>,
    pub target: Interned<String>,
    pub path: PathBuf,
}

struct StepDescription {
    default: bool,
    only_hosts: bool,
    should_run: fn(ShouldRun) -> ShouldRun,
    make_run: fn(RunConfig),
    name: &'static str,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum PathSet {
    Set(BTreeSet<PathBuf>),
    Suite(PathBuf),
}

impl PathSet {
    fn empty() -> PathSet {
        PathSet::Set(BTreeSet::new())
    }

    fn one<P: Into<PathBuf>>(path: P) -> PathSet {
        let mut set = BTreeSet::new();
        set.insert(path.into());
        PathSet::Set(set)
    }

    fn has(&self, needle: &Path) -> bool {
        match self {
            PathSet::Set(set) => set.iter().any(|p| p.ends_with(needle)),
            PathSet::Suite(suite) => suite.ends_with(needle),
        }
    }

    fn path(&self, builder: &Builder) -> PathBuf {
        match self {
            PathSet::Set(set) => set
                .iter()
                .next()
                .unwrap_or(&builder.build.src)
                .to_path_buf(),
            PathSet::Suite(path) => PathBuf::from(path),
        }
    }
}

impl StepDescription {
    fn from<S: Step>() -> StepDescription {
        StepDescription {
            default: S::DEFAULT,
            only_hosts: S::ONLY_HOSTS,
            should_run: S::should_run,
            make_run: S::make_run,
            name: unsafe { ::std::intrinsics::type_name::<S>() },
        }
    }

    fn maybe_run(&self, builder: &Builder, pathset: &PathSet) {
        if builder.config.exclude.iter().any(|e| pathset.has(e)) {
            eprintln!("Skipping {:?} because it is excluded", pathset);
            return;
        } else if !builder.config.exclude.is_empty() {
            eprintln!(
                "{:?} not skipped for {:?} -- not in {:?}",
                pathset, self.name, builder.config.exclude
            );
        }
        let hosts = &builder.hosts;

        // Determine the targets participating in this rule.
        let targets = if self.only_hosts {
            if !builder.config.run_host_only {
                return; // don't run anything
            } else {
                &builder.hosts
            }
        } else {
            &builder.targets
        };

        for host in hosts {
            for target in targets {
                let run = RunConfig {
                    builder,
                    path: pathset.path(builder),
                    host: *host,
                    target: *target,
                };
                (self.make_run)(run);
            }
        }
    }

    fn run(v: &[StepDescription], builder: &Builder, paths: &[PathBuf]) {
        let should_runs = v
            .iter()
            .map(|desc| (desc.should_run)(ShouldRun::new(builder)))
            .collect::<Vec<_>>();

        // sanity checks on rules
        for (desc, should_run) in v.iter().zip(&should_runs) {
            assert!(
                !should_run.paths.is_empty(),
                "{:?} should have at least one pathset",
                desc.name
            );
        }

        if paths.is_empty() {
            for (desc, should_run) in v.iter().zip(should_runs) {
                if desc.default && should_run.is_really_default {
                    for pathset in &should_run.paths {
                        desc.maybe_run(builder, pathset);
                    }
                }
            }
        } else {
            for path in paths {
                // strip CurDir prefix if present
                let path = match path.strip_prefix(".") {
                    Ok(p) => p,
                    Err(_) => path,
                };

                let mut attempted_run = false;
                for (desc, should_run) in v.iter().zip(&should_runs) {
                    if let Some(suite) = should_run.is_suite_path(path) {
                        attempted_run = true;
                        desc.maybe_run(builder, suite);
                    } else if let Some(pathset) = should_run.pathset_for_path(path) {
                        attempted_run = true;
                        desc.maybe_run(builder, pathset);
                    }
                }

                if !attempted_run {
                    panic!("Error: no rules matched {}.", path.display());
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ShouldRun<'a> {
    pub builder: &'a Builder<'a>,
    // use a BTreeSet to maintain sort order
    paths: BTreeSet<PathSet>,

    // If this is a default rule, this is an additional constraint placed on
    // its run. Generally something like compiler docs being enabled.
    is_really_default: bool,
}

impl<'a> ShouldRun<'a> {
    fn new(builder: &'a Builder) -> ShouldRun<'a> {
        ShouldRun {
            builder,
            paths: BTreeSet::new(),
            is_really_default: true, // by default no additional conditions
        }
    }

    pub fn default_condition(mut self, cond: bool) -> Self {
        self.is_really_default = cond;
        self
    }

    // Unlike `krate` this will create just one pathset. As such, it probably shouldn't actually
    // ever be used, but as we transition to having all rules properly handle passing krate(...) by
    // actually doing something different for every crate passed.
    pub fn all_krates(mut self, name: &str) -> Self {
        let mut set = BTreeSet::new();
        for krate in self.builder.in_tree_crates(name) {
            set.insert(PathBuf::from(&krate.path));
        }
        self.paths.insert(PathSet::Set(set));
        self
    }

    pub fn krate(mut self, name: &str) -> Self {
        for krate in self.builder.in_tree_crates(name) {
            self.paths.insert(PathSet::one(&krate.path));
        }
        self
    }

    // single, non-aliased path
    pub fn path(self, path: &str) -> Self {
        self.paths(&[path])
    }

    // multiple aliases for the same job
    pub fn paths(mut self, paths: &[&str]) -> Self {
        self.paths
            .insert(PathSet::Set(paths.iter().map(PathBuf::from).collect()));
        self
    }

    pub fn is_suite_path(&self, path: &Path) -> Option<&PathSet> {
        self.paths.iter().find(|pathset| match pathset {
            PathSet::Suite(p) => path.starts_with(p),
            PathSet::Set(_) => false,
        })
    }

    pub fn suite_path(mut self, suite: &str) -> Self {
        self.paths.insert(PathSet::Suite(PathBuf::from(suite)));
        self
    }

    // allows being more explicit about why should_run in Step returns the value passed to it
    pub fn never(mut self) -> ShouldRun<'a> {
        self.paths.insert(PathSet::empty());
        self
    }

    fn pathset_for_path(&self, path: &Path) -> Option<&PathSet> {
        self.paths.iter().find(|pathset| pathset.has(path))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    Build,
    Check,
    Test,
    Bench,
    Dist,
    Doc,
    Install,
}

impl<'a> Builder<'a> {
    fn get_step_descriptions(kind: Kind) -> Vec<StepDescription> {
        macro_rules! describe {
            ($($rule:ty),+ $(,)*) => {{
                vec![$(StepDescription::from::<$rule>()),+]
            }};
        }
        match kind {
            Kind::Build => describe!(
                compile::Std,
                compile::Test,
                compile::Rustc,
                compile::CodegenBackend,
                compile::StartupObjects,
                tool::BuildManifest,
                tool::Rustbook,
                tool::ErrorIndex,
                tool::UnstableBookGen,
                tool::Tidy,
                tool::Linkchecker,
                tool::CargoTest,
                tool::Compiletest,
                tool::RemoteTestServer,
                tool::RemoteTestClient,
                tool::RustInstaller,
                tool::Cargo,
                tool::Rls,
                tool::Rustdoc,
                tool::Clippy,
                native::Llvm,
                tool::Rustfmt,
                tool::Miri,
                native::Lld
            ),
            Kind::Check => describe!(
                check::Std,
                check::Test,
                check::Rustc,
                check::CodegenBackend,
                check::Rustdoc
            ),
            Kind::Test => describe!(
                test::Tidy,
                test::Ui,
                test::RunPass,
                test::CompileFail,
                test::RunFail,
                test::RunPassValgrind,
                test::MirOpt,
                test::Codegen,
                test::CodegenUnits,
                test::Incremental,
                test::Debuginfo,
                test::UiFullDeps,
                test::RunPassFullDeps,
                test::RunFailFullDeps,
                test::Rustdoc,
                test::Pretty,
                test::RunPassPretty,
                test::RunFailPretty,
                test::RunPassValgrindPretty,
                test::RunPassFullDepsPretty,
                test::RunFailFullDepsPretty,
                test::Crate,
                test::CrateLibrustc,
                test::CrateRustdoc,
                test::Linkcheck,
                test::Cargotest,
                test::Cargo,
                test::Rls,
                test::ErrorIndex,
                test::Distcheck,
                test::RunMakeFullDeps,
                test::Nomicon,
                test::Reference,
                test::RustdocBook,
                test::RustByExample,
                test::TheBook,
                test::UnstableBook,
                test::RustcBook,
                test::Rustfmt,
                test::Miri,
                test::Clippy,
                test::CompiletestTest,
                test::RustdocJS,
                test::RustdocTheme,
                // Run bootstrap close to the end as it's unlikely to fail
                test::Bootstrap,
                // Run run-make last, since these won't pass without make on Windows
                test::RunMake,
                test::RustdocUi
            ),
            Kind::Bench => describe!(test::Crate, test::CrateLibrustc),
            Kind::Doc => describe!(
                doc::UnstableBook,
                doc::UnstableBookGen,
                doc::TheBook,
                doc::Standalone,
                doc::Std,
                doc::Test,
                doc::WhitelistedRustc,
                doc::Rustc,
                doc::Rustdoc,
                doc::ErrorIndex,
                doc::Nomicon,
                doc::Reference,
                doc::RustdocBook,
                doc::RustByExample,
                doc::RustcBook,
                doc::CargoBook,
                doc::EditionGuide,
            ),
            Kind::Dist => describe!(
                dist::Docs,
                dist::RustcDocs,
                dist::Mingw,
                dist::Rustc,
                dist::DebuggerScripts,
                dist::Std,
                dist::Analysis,
                dist::Src,
                dist::PlainSourceTarball,
                dist::Cargo,
                dist::Rls,
                dist::Rustfmt,
                dist::Clippy,
                dist::LlvmTools,
                dist::Lldb,
                dist::Extended,
                dist::HashSign
            ),
            Kind::Install => describe!(
                install::Docs,
                install::Std,
                install::Cargo,
                install::Rls,
                install::Rustfmt,
                install::Clippy,
                install::Analysis,
                install::Src,
                install::Rustc
            ),
        }
    }

    pub fn get_help(build: &Build, subcommand: &str) -> Option<String> {
        let kind = match subcommand {
            "build" => Kind::Build,
            "doc" => Kind::Doc,
            "test" => Kind::Test,
            "bench" => Kind::Bench,
            "dist" => Kind::Dist,
            "install" => Kind::Install,
            _ => return None,
        };

        let builder = Builder {
            build,
            top_stage: build.config.stage.unwrap_or(2),
            kind,
            cache: Cache::new(),
            stack: RefCell::new(Vec::new()),
            time_spent_on_dependencies: Cell::new(Duration::new(0, 0)),
            paths: vec![],
            graph_nodes: RefCell::new(HashMap::new()),
            graph: RefCell::new(Graph::new()),
            parent: Cell::new(None),
        };

        let builder = &builder;
        let mut should_run = ShouldRun::new(builder);
        for desc in Builder::get_step_descriptions(builder.kind) {
            should_run = (desc.should_run)(should_run);
        }
        let mut help = String::from("Available paths:\n");
        for pathset in should_run.paths {
            if let PathSet::Set(set) = pathset {
                set.iter().for_each(|path| {
                    help.push_str(
                        format!("    ./x.py {} {}\n", subcommand, path.display()).as_str(),
                    )
                })
            }
        }
        Some(help)
    }

    pub fn new(build: &Build) -> Builder {
        let (kind, paths) = match build.config.cmd {
            Subcommand::Build { ref paths } => (Kind::Build, &paths[..]),
            Subcommand::Check { ref paths } => (Kind::Check, &paths[..]),
            Subcommand::Doc { ref paths } => (Kind::Doc, &paths[..]),
            Subcommand::Test { ref paths, .. } => (Kind::Test, &paths[..]),
            Subcommand::Bench { ref paths, .. } => (Kind::Bench, &paths[..]),
            Subcommand::Dist { ref paths } => (Kind::Dist, &paths[..]),
            Subcommand::Install { ref paths } => (Kind::Install, &paths[..]),
            Subcommand::Clean { .. } => panic!(),
        };

        let builder = Builder {
            build,
            top_stage: build.config.stage.unwrap_or(2),
            kind,
            cache: Cache::new(),
            stack: RefCell::new(Vec::new()),
            time_spent_on_dependencies: Cell::new(Duration::new(0, 0)),
            paths: paths.to_owned(),
            graph_nodes: RefCell::new(HashMap::new()),
            graph: RefCell::new(Graph::new()),
            parent: Cell::new(None),
        };

        if kind == Kind::Dist {
            assert!(
                !builder.config.test_miri,
                "Do not distribute with miri enabled.\n\
                The distributed libraries would include all MIR (increasing binary size).
                The distributed MIR would include validation statements."
            );
        }

        builder
    }

    pub fn execute_cli(&self) -> Graph<String, bool> {
        self.run_step_descriptions(&Builder::get_step_descriptions(self.kind), &self.paths);
        self.graph.borrow().clone()
    }

    pub fn default_doc(&self, paths: Option<&[PathBuf]>) {
        let paths = paths.unwrap_or(&[]);
        self.run_step_descriptions(&Builder::get_step_descriptions(Kind::Doc), paths);
    }

    fn run_step_descriptions(&self, v: &[StepDescription], paths: &[PathBuf]) {
        StepDescription::run(v, self, paths);
    }

    /// Obtain a compiler at a given stage and for a given host. Explicitly does
    /// not take `Compiler` since all `Compiler` instances are meant to be
    /// obtained through this function, since it ensures that they are valid
    /// (i.e., built and assembled).
    pub fn compiler(&self, stage: u32, host: Interned<String>) -> Compiler {
        self.ensure(compile::Assemble {
            target_compiler: Compiler { stage, host },
        })
    }

    pub fn sysroot(&self, compiler: Compiler) -> Interned<PathBuf> {
        self.ensure(compile::Sysroot { compiler })
    }

    /// Returns the libdir where the standard library and other artifacts are
    /// found for a compiler's sysroot.
    pub fn sysroot_libdir(
        &self,
        compiler: Compiler,
        target: Interned<String>,
    ) -> Interned<PathBuf> {
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        struct Libdir {
            compiler: Compiler,
            target: Interned<String>,
        }
        impl Step for Libdir {
            type Output = Interned<PathBuf>;

            fn should_run(run: ShouldRun) -> ShouldRun {
                run.never()
            }

            fn run(self, builder: &Builder) -> Interned<PathBuf> {
                let compiler = self.compiler;
                let config = &builder.build.config;
                let lib = if compiler.stage >= 1 && config.libdir_relative().is_some() {
                    builder.build.config.libdir_relative().unwrap()
                } else {
                    Path::new("lib")
                };
                let sysroot = builder
                    .sysroot(self.compiler)
                    .join(lib)
                    .join("rustlib")
                    .join(self.target)
                    .join("lib");
                let _ = fs::remove_dir_all(&sysroot);
                t!(fs::create_dir_all(&sysroot));
                INTERNER.intern_path(sysroot)
            }
        }
        self.ensure(Libdir { compiler, target })
    }

    pub fn sysroot_codegen_backends(&self, compiler: Compiler) -> PathBuf {
        self.sysroot_libdir(compiler, compiler.host)
            .with_file_name(self.config.rust_codegen_backends_dir.clone())
    }

    /// Returns the compiler's libdir where it stores the dynamic libraries that
    /// it itself links against.
    ///
    /// For example this returns `<sysroot>/lib` on Unix and `<sysroot>/bin` on
    /// Windows.
    pub fn rustc_libdir(&self, compiler: Compiler) -> PathBuf {
        if compiler.is_snapshot(self) {
            self.rustc_snapshot_libdir()
        } else {
            self.sysroot(compiler).join(libdir(&compiler.host))
        }
    }

    /// Adds the compiler's directory of dynamic libraries to `cmd`'s dynamic
    /// library lookup path.
    pub fn add_rustc_lib_path(&self, compiler: Compiler, cmd: &mut Command) {
        // Windows doesn't need dylib path munging because the dlls for the
        // compiler live next to the compiler and the system will find them
        // automatically.
        if cfg!(windows) {
            return;
        }

        add_lib_path(vec![self.rustc_libdir(compiler)], cmd);
    }

    /// Get a path to the compiler specified.
    pub fn rustc(&self, compiler: Compiler) -> PathBuf {
        if compiler.is_snapshot(self) {
            self.initial_rustc.clone()
        } else {
            self.sysroot(compiler)
                .join("bin")
                .join(exe("rustc", &compiler.host))
        }
    }

    pub fn rustdoc(&self, host: Interned<String>) -> PathBuf {
        self.ensure(tool::Rustdoc { host })
    }

    pub fn rustdoc_cmd(&self, host: Interned<String>) -> Command {
        let mut cmd = Command::new(&self.out.join("bootstrap/debug/rustdoc"));
        let compiler = self.compiler(self.top_stage, host);
        cmd.env("RUSTC_STAGE", compiler.stage.to_string())
            .env("RUSTC_SYSROOT", self.sysroot(compiler))
            .env(
                "RUSTDOC_LIBDIR",
                self.sysroot_libdir(compiler, self.config.build),
            )
            .env("CFG_RELEASE_CHANNEL", &self.config.channel)
            .env("RUSTDOC_REAL", self.rustdoc(host))
            .env("RUSTDOC_CRATE_VERSION", self.rust_version())
            .env("RUSTC_BOOTSTRAP", "1");

        // Remove make-related flags that can cause jobserver problems.
        cmd.env_remove("MAKEFLAGS");
        cmd.env_remove("MFLAGS");

        if let Some(linker) = self.linker(host) {
            cmd.env("RUSTC_TARGET_LINKER", linker);
        }
        cmd
    }

    /// Prepares an invocation of `cargo` to be run.
    ///
    /// This will create a `Command` that represents a pending execution of
    /// Cargo. This cargo will be configured to use `compiler` as the actual
    /// rustc compiler, its output will be scoped by `mode`'s output directory,
    /// it will pass the `--target` flag for the specified `target`, and will be
    /// executing the Cargo command `cmd`.
    pub fn cargo(
        &self,
        compiler: Compiler,
        mode: Mode,
        target: Interned<String>,
        cmd: &str,
    ) -> Command {
        let mut cargo = Command::new(&self.initial_cargo);
        let out_dir = self.stage_out(compiler, mode);

        // command specific path, we call clear_if_dirty with this
        let mut my_out = match cmd {
            "build" => self.cargo_out(compiler, mode, target),

            // This is the intended out directory for crate documentation.
            "doc" | "rustdoc" =>  self.crate_doc_out(target),

            _ => self.stage_out(compiler, mode),
        };

        // This is for the original compiler, but if we're forced to use stage 1, then
        // std/test/rustc stamps won't exist in stage 2, so we need to get those from stage 1, since
        // we copy the libs forward.
        let cmp = if self.force_use_stage1(compiler, target) {
            self.compiler(1, compiler.host)
        } else {
            compiler
        };

        let libstd_stamp = match cmd {
            "check" => check::libstd_stamp(self, cmp, target),
            _ => compile::libstd_stamp(self, cmp, target),
        };

        let libtest_stamp = match cmd {
            "check" => check::libtest_stamp(self, cmp, target),
            _ => compile::libstd_stamp(self, cmp, target),
        };

        let librustc_stamp = match cmd {
            "check" => check::librustc_stamp(self, cmp, target),
            _ => compile::librustc_stamp(self, cmp, target),
        };

        if cmd == "doc" || cmd == "rustdoc" {
            if mode == Mode::Rustc || mode == Mode::ToolRustc || mode == Mode::Codegen {
                // This is the intended out directory for compiler documentation.
                my_out = self.compiler_doc_out(target);
            }
            let rustdoc = self.rustdoc(compiler.host);
            self.clear_if_dirty(&my_out, &rustdoc);
        } else if cmd != "test" {
            match mode {
                Mode::Std => {
                    self.clear_if_dirty(&my_out, &self.rustc(compiler));
                },
                Mode::Test => {
                    self.clear_if_dirty(&my_out, &libstd_stamp);
                },
                Mode::Rustc => {
                    self.clear_if_dirty(&my_out, &self.rustc(compiler));
                    self.clear_if_dirty(&my_out, &libstd_stamp);
                    self.clear_if_dirty(&my_out, &libtest_stamp);
                },
                Mode::Codegen => {
                    self.clear_if_dirty(&my_out, &librustc_stamp);
                },
                Mode::ToolBootstrap => { },
                Mode::ToolStd => {
                    self.clear_if_dirty(&my_out, &libstd_stamp);
                },
                Mode::ToolTest => {
                    self.clear_if_dirty(&my_out, &libstd_stamp);
                    self.clear_if_dirty(&my_out, &libtest_stamp);
                },
                Mode::ToolRustc => {
                    self.clear_if_dirty(&my_out, &libstd_stamp);
                    self.clear_if_dirty(&my_out, &libtest_stamp);
                    self.clear_if_dirty(&my_out, &librustc_stamp);
                },
            }
        }

        cargo
            .env("CARGO_TARGET_DIR", out_dir)
            .arg(cmd);

        if cmd != "install" {
            cargo.arg("--target")
                 .arg(target);
        } else {
            assert_eq!(target, compiler.host);
        }

        // Set a flag for `check` so that certain build scripts can do less work
        // (e.g., not building/requiring LLVM).
        if cmd == "check" {
            cargo.env("RUST_CHECK", "1");
        }

        cargo.arg("-j").arg(self.jobs().to_string());
        // Remove make-related flags to ensure Cargo can correctly set things up
        cargo.env_remove("MAKEFLAGS");
        cargo.env_remove("MFLAGS");

        // FIXME: Temporary fix for https://github.com/rust-lang/cargo/issues/3005
        // Force cargo to output binaries with disambiguating hashes in the name
        let metadata = if compiler.stage == 0 {
            // Treat stage0 like special channel, whether it's a normal prior-
            // release rustc or a local rebuild with the same version, so we
            // never mix these libraries by accident.
            "bootstrap"
        } else {
            &self.config.channel
        };
        cargo.env("__CARGO_DEFAULT_LIB_METADATA", &metadata);

        let stage;
        if compiler.stage == 0 && self.local_rebuild {
            // Assume the local-rebuild rustc already has stage1 features.
            stage = 1;
        } else {
            stage = compiler.stage;
        }

        let mut extra_args = env::var(&format!("RUSTFLAGS_STAGE_{}", stage)).unwrap_or_default();
        if stage != 0 {
            let s = env::var("RUSTFLAGS_STAGE_NOT_0").unwrap_or_default();
            if !extra_args.is_empty() {
                extra_args.push_str(" ");
            }
            extra_args.push_str(&s);
        }

        if !extra_args.is_empty() {
            cargo.env(
                "RUSTFLAGS",
                format!(
                    "{} {}",
                    env::var("RUSTFLAGS").unwrap_or_default(),
                    extra_args
                ),
            );
        }

        let want_rustdoc = self.doc_tests != DocTests::No;

        // We synthetically interpret a stage0 compiler used to build tools as a
        // "raw" compiler in that it's the exact snapshot we download. Normally
        // the stage0 build means it uses libraries build by the stage0
        // compiler, but for tools we just use the precompiled libraries that
        // we've downloaded
        let use_snapshot = mode == Mode::ToolBootstrap;
        assert!(!use_snapshot || stage == 0 || self.local_rebuild);

        let maybe_sysroot = self.sysroot(compiler);
        let sysroot = if use_snapshot {
            self.rustc_snapshot_sysroot()
        } else {
            &maybe_sysroot
        };
        let libdir = sysroot.join(libdir(&compiler.host));

        // Customize the compiler we're running. Specify the compiler to cargo
        // as our shim and then pass it some various options used to configure
        // how the actual compiler itself is called.
        //
        // These variables are primarily all read by
        // src/bootstrap/bin/{rustc.rs,rustdoc.rs}
        cargo
            .env("RUSTBUILD_NATIVE_DIR", self.native_dir(target))
            .env("RUSTC", self.out.join("bootstrap/debug/rustc"))
            .env("RUSTC_REAL", self.rustc(compiler))
            .env("RUSTC_STAGE", stage.to_string())
            .env(
                "RUSTC_DEBUG_ASSERTIONS",
                self.config.rust_debug_assertions.to_string(),
            )
            .env("RUSTC_SYSROOT", &sysroot)
            .env("RUSTC_LIBDIR", &libdir)
            .env("RUSTC_RPATH", self.config.rust_rpath.to_string())
            .env("RUSTDOC", self.out.join("bootstrap/debug/rustdoc"))
            .env(
                "RUSTDOC_REAL",
                if cmd == "doc" || cmd == "rustdoc" || (cmd == "test" && want_rustdoc) {
                    self.rustdoc(compiler.host)
                } else {
                    PathBuf::from("/path/to/nowhere/rustdoc/not/required")
                },
            )
            .env("TEST_MIRI", self.config.test_miri.to_string())
            .env("RUSTC_ERROR_METADATA_DST", self.extended_error_dir());

        if let Some(host_linker) = self.linker(compiler.host) {
            cargo.env("RUSTC_HOST_LINKER", host_linker);
        }
        if let Some(target_linker) = self.linker(target) {
            cargo.env("RUSTC_TARGET_LINKER", target_linker);
        }
        if let Some(ref error_format) = self.config.rustc_error_format {
            cargo.env("RUSTC_ERROR_FORMAT", error_format);
        }
        if cmd != "build" && cmd != "check" && cmd != "rustc" && want_rustdoc {
            cargo.env("RUSTDOC_LIBDIR", self.sysroot_libdir(compiler, self.config.build));
        }

        if mode.is_tool() {
            // Tools like cargo and rls don't get debuginfo by default right now, but this can be
            // enabled in the config.  Adding debuginfo makes them several times larger.
            if self.config.rust_debuginfo_tools {
                cargo.env("RUSTC_DEBUGINFO", self.config.rust_debuginfo.to_string());
                cargo.env(
                    "RUSTC_DEBUGINFO_LINES",
                    self.config.rust_debuginfo_lines.to_string(),
                );
            }
        } else {
            cargo.env("RUSTC_DEBUGINFO", self.config.rust_debuginfo.to_string());
            cargo.env(
                "RUSTC_DEBUGINFO_LINES",
                self.config.rust_debuginfo_lines.to_string(),
            );
            cargo.env("RUSTC_FORCE_UNSTABLE", "1");

            // Currently the compiler depends on crates from crates.io, and
            // then other crates can depend on the compiler (e.g., proc-macro
            // crates). Let's say, for example that rustc itself depends on the
            // bitflags crate. If an external crate then depends on the
            // bitflags crate as well, we need to make sure they don't
            // conflict, even if they pick the same version of bitflags. We'll
            // want to make sure that e.g., a plugin and rustc each get their
            // own copy of bitflags.

            // Cargo ensures that this works in general through the -C metadata
            // flag. This flag will frob the symbols in the binary to make sure
            // they're different, even though the source code is the exact
            // same. To solve this problem for the compiler we extend Cargo's
            // already-passed -C metadata flag with our own. Our rustc.rs
            // wrapper around the actual rustc will detect -C metadata being
            // passed and frob it with this extra string we're passing in.
            cargo.env("RUSTC_METADATA_SUFFIX", "rustc");
        }

        if let Some(x) = self.crt_static(target) {
            cargo.env("RUSTC_CRT_STATIC", x.to_string());
        }

        if let Some(x) = self.crt_static(compiler.host) {
            cargo.env("RUSTC_HOST_CRT_STATIC", x.to_string());
        }

        if let Some(map) = self.build.debuginfo_map(GitRepo::Rustc) {
            cargo.env("RUSTC_DEBUGINFO_MAP", map);
        }

        // Enable usage of unstable features
        cargo.env("RUSTC_BOOTSTRAP", "1");
        self.add_rust_test_threads(&mut cargo);

        // Almost all of the crates that we compile as part of the bootstrap may
        // have a build script, including the standard library. To compile a
        // build script, however, it itself needs a standard library! This
        // introduces a bit of a pickle when we're compiling the standard
        // library itself.
        //
        // To work around this we actually end up using the snapshot compiler
        // (stage0) for compiling build scripts of the standard library itself.
        // The stage0 compiler is guaranteed to have a libstd available for use.
        //
        // For other crates, however, we know that we've already got a standard
        // library up and running, so we can use the normal compiler to compile
        // build scripts in that situation.
        //
        // If LLVM support is disabled we need to use the snapshot compiler to compile
        // build scripts, as the new compiler doesn't support executables.
        if mode == Mode::Std || !self.config.llvm_enabled {
            cargo
                .env("RUSTC_SNAPSHOT", &self.initial_rustc)
                .env("RUSTC_SNAPSHOT_LIBDIR", self.rustc_snapshot_libdir());
        } else {
            cargo
                .env("RUSTC_SNAPSHOT", self.rustc(compiler))
                .env("RUSTC_SNAPSHOT_LIBDIR", self.rustc_libdir(compiler));
        }

        if self.config.incremental {
            cargo.env("CARGO_INCREMENTAL", "1");
        }

        if let Some(ref on_fail) = self.config.on_fail {
            cargo.env("RUSTC_ON_FAIL", on_fail);
        }

        if self.config.print_step_timings {
            cargo.env("RUSTC_PRINT_STEP_TIMINGS", "1");
        }

        if self.config.backtrace_on_ice {
            cargo.env("RUSTC_BACKTRACE_ON_ICE", "1");
        }

        cargo.env("RUSTC_VERBOSE", self.verbosity.to_string());

        // in std, we want to avoid denying warnings for stage 0 as that makes cfg's painful.
        if self.config.deny_warnings && !(mode == Mode::Std && stage == 0) {
            cargo.env("RUSTC_DENY_WARNINGS", "1");
        }

        // Throughout the build Cargo can execute a number of build scripts
        // compiling C/C++ code and we need to pass compilers, archivers, flags, etc
        // obtained previously to those build scripts.
        // Build scripts use either the `cc` crate or `configure/make` so we pass
        // the options through environment variables that are fetched and understood by both.
        //
        // FIXME: the guard against msvc shouldn't need to be here
        if target.contains("msvc") {
            if let Some(ref cl) = self.config.llvm_clang_cl {
                cargo.env("CC", cl).env("CXX", cl);
            }
        } else {
            let ccache = self.config.ccache.as_ref();
            let ccacheify = |s: &Path| {
                let ccache = match ccache {
                    Some(ref s) => s,
                    None => return s.display().to_string(),
                };
                // FIXME: the cc-rs crate only recognizes the literal strings
                // `ccache` and `sccache` when doing caching compilations, so we
                // mirror that here. It should probably be fixed upstream to
                // accept a new env var or otherwise work with custom ccache
                // vars.
                match &ccache[..] {
                    "ccache" | "sccache" => format!("{} {}", ccache, s.display()),
                    _ => s.display().to_string(),
                }
            };
            let cc = ccacheify(&self.cc(target));
            cargo.env(format!("CC_{}", target), &cc).env("CC", &cc);

            let cflags = self.cflags(target, GitRepo::Rustc).join(" ");
            cargo
                .env(format!("CFLAGS_{}", target), cflags.clone())
                .env("CFLAGS", cflags.clone());

            if let Some(ar) = self.ar(target) {
                let ranlib = format!("{} s", ar.display());
                cargo
                    .env(format!("AR_{}", target), ar)
                    .env("AR", ar)
                    .env(format!("RANLIB_{}", target), ranlib.clone())
                    .env("RANLIB", ranlib);
            }

            if let Ok(cxx) = self.cxx(target) {
                let cxx = ccacheify(&cxx);
                cargo
                    .env(format!("CXX_{}", target), &cxx)
                    .env("CXX", &cxx)
                    .env(format!("CXXFLAGS_{}", target), cflags.clone())
                    .env("CXXFLAGS", cflags);
            }
        }

        if (cmd == "build" || cmd == "rustc")
            && mode == Mode::Std
            && self.config.extended
            && compiler.is_final_stage(self)
        {
            cargo.env("RUSTC_SAVE_ANALYSIS", "api".to_string());
        }

        // For `cargo doc` invocations, make rustdoc print the Rust version into the docs
        cargo.env("RUSTDOC_CRATE_VERSION", self.rust_version());

        // Environment variables *required* throughout the build
        //
        // FIXME: should update code to not require this env var
        cargo.env("CFG_COMPILER_HOST_TRIPLE", target);

        // Set this for all builds to make sure doc builds also get it.
        cargo.env("CFG_RELEASE_CHANNEL", &self.config.channel);

        // This one's a bit tricky. As of the time of this writing the compiler
        // links to the `winapi` crate on crates.io. This crate provides raw
        // bindings to Windows system functions, sort of like libc does for
        // Unix. This crate also, however, provides "import libraries" for the
        // MinGW targets. There's an import library per dll in the windows
        // distribution which is what's linked to. These custom import libraries
        // are used because the winapi crate can reference Windows functions not
        // present in the MinGW import libraries.
        //
        // For example MinGW may ship libdbghelp.a, but it may not have
        // references to all the functions in the dbghelp dll. Instead the
        // custom import library for dbghelp in the winapi crates has all this
        // information.
        //
        // Unfortunately for us though the import libraries are linked by
        // default via `-ldylib=winapi_foo`. That is, they're linked with the
        // `dylib` type with a `winapi_` prefix (so the winapi ones don't
        // conflict with the system MinGW ones). This consequently means that
        // the binaries we ship of things like rustc_codegen_llvm (aka the rustc_codegen_llvm
        // DLL) when linked against *again*, for example with procedural macros
        // or plugins, will trigger the propagation logic of `-ldylib`, passing
        // `-lwinapi_foo` to the linker again. This isn't actually available in
        // our distribution, however, so the link fails.
        //
        // To solve this problem we tell winapi to not use its bundled import
        // libraries. This means that it will link to the system MinGW import
        // libraries by default, and the `-ldylib=foo` directives will still get
        // passed to the final linker, but they'll look like `-lfoo` which can
        // be resolved because MinGW has the import library. The downside is we
        // don't get newer functions from Windows, but we don't use any of them
        // anyway.
        if !mode.is_tool() {
            cargo.env("WINAPI_NO_BUNDLED_LIBRARIES", "1");
        }

        for _ in 1..self.verbosity {
            cargo.arg("-v");
        }

        match (mode, self.config.rust_codegen_units_std, self.config.rust_codegen_units) {
            (Mode::Std, Some(n), _) |
            (Mode::Test, Some(n), _) |
            (_, _, Some(n)) => {
                cargo.env("RUSTC_CODEGEN_UNITS", n.to_string());
            }
            _ => {
                // Don't set anything
            }
        }

        if self.config.rust_optimize {
            // FIXME: cargo bench/install do not accept `--release`
            if cmd != "bench" && cmd != "install" {
                cargo.arg("--release");
            }
        }

        if self.config.locked_deps {
            cargo.arg("--locked");
        }
        if self.config.vendor || self.is_sudo {
            cargo.arg("--frozen");
        }

        self.ci_env.force_coloring_in_ci(&mut cargo);

        cargo
    }

    /// Ensure that a given step is built, returning its output. This will
    /// cache the step, so it is safe (and good!) to call this as often as
    /// needed to ensure that all dependencies are built.
    pub fn ensure<S: Step>(&'a self, step: S) -> S::Output {
        {
            let mut stack = self.stack.borrow_mut();
            for stack_step in stack.iter() {
                // should skip
                if stack_step
                    .downcast_ref::<S>()
                    .map_or(true, |stack_step| *stack_step != step)
                {
                    continue;
                }
                let mut out = String::new();
                out += &format!("\n\nCycle in build detected when adding {:?}\n", step);
                for el in stack.iter().rev() {
                    out += &format!("\t{:?}\n", el);
                }
                panic!(out);
            }
            if let Some(out) = self.cache.get(&step) {
                self.verbose(&format!("{}c {:?}", "  ".repeat(stack.len()), step));

                {
                    let mut graph = self.graph.borrow_mut();
                    let parent = self.parent.get();
                    let us = *self
                        .graph_nodes
                        .borrow_mut()
                        .entry(format!("{:?}", step))
                        .or_insert_with(|| graph.add_node(format!("{:?}", step)));
                    if let Some(parent) = parent {
                        graph.add_edge(parent, us, false);
                    }
                }

                return out;
            }
            self.verbose(&format!("{}> {:?}", "  ".repeat(stack.len()), step));
            stack.push(Box::new(step.clone()));
        }

        let prev_parent = self.parent.get();

        {
            let mut graph = self.graph.borrow_mut();
            let parent = self.parent.get();
            let us = *self
                .graph_nodes
                .borrow_mut()
                .entry(format!("{:?}", step))
                .or_insert_with(|| graph.add_node(format!("{:?}", step)));
            self.parent.set(Some(us));
            if let Some(parent) = parent {
                graph.add_edge(parent, us, true);
            }
        }

        let (out, dur) = {
            let start = Instant::now();
            let zero = Duration::new(0, 0);
            let parent = self.time_spent_on_dependencies.replace(zero);
            let out = step.clone().run(self);
            let dur = start.elapsed();
            let deps = self.time_spent_on_dependencies.replace(parent + dur);
            (out, dur - deps)
        };

        self.parent.set(prev_parent);

        if self.config.print_step_timings && dur > Duration::from_millis(100) {
            println!(
                "[TIMING] {:?} -- {}.{:03}",
                step,
                dur.as_secs(),
                dur.subsec_nanos() / 1_000_000
            );
        }

        {
            let mut stack = self.stack.borrow_mut();
            let cur_step = stack.pop().expect("step stack empty");
            assert_eq!(cur_step.downcast_ref(), Some(&step));
        }
        self.verbose(&format!(
            "{}< {:?}",
            "  ".repeat(self.stack.borrow().len()),
            step
        ));
        self.cache.put(step, out.clone());
        out
    }
}

#[cfg(test)]
mod __test {
    use super::*;
    use crate::config::Config;
    use std::thread;

    fn configure(host: &[&str], target: &[&str]) -> Config {
        let mut config = Config::default_opts();
        // don't save toolstates
        config.save_toolstates = None;
        config.run_host_only = true;
        config.dry_run = true;
        // try to avoid spurious failures in dist where we create/delete each others file
        let dir = config.out.join("tmp-rustbuild-tests").join(
            &thread::current()
                .name()
                .unwrap_or("unknown")
                .replace(":", "-"),
        );
        t!(fs::create_dir_all(&dir));
        config.out = dir;
        config.build = INTERNER.intern_str("A");
        config.hosts = vec![config.build]
            .clone()
            .into_iter()
            .chain(host.iter().map(|s| INTERNER.intern_str(s)))
            .collect::<Vec<_>>();
        config.targets = config
            .hosts
            .clone()
            .into_iter()
            .chain(target.iter().map(|s| INTERNER.intern_str(s)))
            .collect::<Vec<_>>();
        config
    }

    fn first<A, B>(v: Vec<(A, B)>) -> Vec<A> {
        v.into_iter().map(|(a, _)| a).collect::<Vec<_>>()
    }

    #[test]
    fn dist_baseline() {
        let build = Build::new(configure(&[], &[]));
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Dist), &[]);

        let a = INTERNER.intern_str("A");

        assert_eq!(
            first(builder.cache.all::<dist::Docs>()),
            &[dist::Docs { stage: 2, host: a },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Mingw>()),
            &[dist::Mingw { host: a },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Rustc>()),
            &[dist::Rustc {
                compiler: Compiler { host: a, stage: 2 }
            },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Std>()),
            &[dist::Std {
                compiler: Compiler { host: a, stage: 2 },
                target: a,
            },]
        );
        assert_eq!(first(builder.cache.all::<dist::Src>()), &[dist::Src]);
    }

    #[test]
    fn dist_with_targets() {
        let build = Build::new(configure(&[], &["B"]));
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Dist), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");

        assert_eq!(
            first(builder.cache.all::<dist::Docs>()),
            &[
                dist::Docs { stage: 2, host: a },
                dist::Docs { stage: 2, host: b },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Mingw>()),
            &[dist::Mingw { host: a }, dist::Mingw { host: b },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Rustc>()),
            &[dist::Rustc {
                compiler: Compiler { host: a, stage: 2 }
            },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Std>()),
            &[
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
            ]
        );
        assert_eq!(first(builder.cache.all::<dist::Src>()), &[dist::Src]);
    }

    #[test]
    fn dist_with_hosts() {
        let build = Build::new(configure(&["B"], &[]));
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Dist), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");

        assert_eq!(
            first(builder.cache.all::<dist::Docs>()),
            &[
                dist::Docs { stage: 2, host: a },
                dist::Docs { stage: 2, host: b },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Mingw>()),
            &[dist::Mingw { host: a }, dist::Mingw { host: b },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Rustc>()),
            &[
                dist::Rustc {
                    compiler: Compiler { host: a, stage: 2 }
                },
                dist::Rustc {
                    compiler: Compiler { host: b, stage: 2 }
                },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Std>()),
            &[
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
            ]
        );
        assert_eq!(first(builder.cache.all::<dist::Src>()), &[dist::Src]);
    }

    #[test]
    fn dist_with_targets_and_hosts() {
        let build = Build::new(configure(&["B"], &["C"]));
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Dist), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");
        let c = INTERNER.intern_str("C");

        assert_eq!(
            first(builder.cache.all::<dist::Docs>()),
            &[
                dist::Docs { stage: 2, host: a },
                dist::Docs { stage: 2, host: b },
                dist::Docs { stage: 2, host: c },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Mingw>()),
            &[
                dist::Mingw { host: a },
                dist::Mingw { host: b },
                dist::Mingw { host: c },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Rustc>()),
            &[
                dist::Rustc {
                    compiler: Compiler { host: a, stage: 2 }
                },
                dist::Rustc {
                    compiler: Compiler { host: b, stage: 2 }
                },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Std>()),
            &[
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: c,
                },
            ]
        );
        assert_eq!(first(builder.cache.all::<dist::Src>()), &[dist::Src]);
    }

    #[test]
    fn dist_with_target_flag() {
        let mut config = configure(&["B"], &["C"]);
        config.run_host_only = false; // as-if --target=C was passed
        let build = Build::new(config);
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Dist), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");
        let c = INTERNER.intern_str("C");

        assert_eq!(
            first(builder.cache.all::<dist::Docs>()),
            &[
                dist::Docs { stage: 2, host: a },
                dist::Docs { stage: 2, host: b },
                dist::Docs { stage: 2, host: c },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Mingw>()),
            &[
                dist::Mingw { host: a },
                dist::Mingw { host: b },
                dist::Mingw { host: c },
            ]
        );
        assert_eq!(first(builder.cache.all::<dist::Rustc>()), &[]);
        assert_eq!(
            first(builder.cache.all::<dist::Std>()),
            &[
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: c,
                },
            ]
        );
        assert_eq!(first(builder.cache.all::<dist::Src>()), &[]);
    }

    #[test]
    fn dist_with_same_targets_and_hosts() {
        let build = Build::new(configure(&["B"], &["B"]));
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Dist), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");

        assert_eq!(
            first(builder.cache.all::<dist::Docs>()),
            &[
                dist::Docs { stage: 2, host: a },
                dist::Docs { stage: 2, host: b },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Mingw>()),
            &[dist::Mingw { host: a }, dist::Mingw { host: b },]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Rustc>()),
            &[
                dist::Rustc {
                    compiler: Compiler { host: a, stage: 2 }
                },
                dist::Rustc {
                    compiler: Compiler { host: b, stage: 2 }
                },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<dist::Std>()),
            &[
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                dist::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
            ]
        );
        assert_eq!(first(builder.cache.all::<dist::Src>()), &[dist::Src]);
        assert_eq!(
            first(builder.cache.all::<compile::Std>()),
            &[
                compile::Std {
                    compiler: Compiler { host: a, stage: 0 },
                    target: a,
                },
                compile::Std {
                    compiler: Compiler { host: a, stage: 1 },
                    target: a,
                },
                compile::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                compile::Std {
                    compiler: Compiler { host: a, stage: 1 },
                    target: b,
                },
                compile::Std {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<compile::Test>()),
            &[
                compile::Test {
                    compiler: Compiler { host: a, stage: 0 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 1 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 1 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<compile::Assemble>()),
            &[
                compile::Assemble {
                    target_compiler: Compiler { host: a, stage: 0 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: a, stage: 1 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: a, stage: 2 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: b, stage: 2 },
                },
            ]
        );
    }

    #[test]
    fn build_default() {
        let build = Build::new(configure(&["B"], &["C"]));
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Build), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");
        let c = INTERNER.intern_str("C");

        assert!(!builder.cache.all::<compile::Std>().is_empty());
        assert!(!builder.cache.all::<compile::Assemble>().is_empty());
        assert_eq!(
            first(builder.cache.all::<compile::Rustc>()),
            &[
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 0 },
                    target: a,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 1 },
                    target: a,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                compile::Rustc {
                    compiler: Compiler { host: b, stage: 2 },
                    target: a,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 0 },
                    target: b,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 1 },
                    target: b,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
                compile::Rustc {
                    compiler: Compiler { host: b, stage: 2 },
                    target: b,
                },
            ]
        );

        assert_eq!(
            first(builder.cache.all::<compile::Test>()),
            &[
                compile::Test {
                    compiler: Compiler { host: a, stage: 0 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 1 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: b, stage: 2 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 0 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 1 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: b, stage: 2 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: c,
                },
                compile::Test {
                    compiler: Compiler { host: b, stage: 2 },
                    target: c,
                },
            ]
        );
    }

    #[test]
    fn build_with_target_flag() {
        let mut config = configure(&["B"], &["C"]);
        config.run_host_only = false;
        let build = Build::new(config);
        let mut builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Build), &[]);

        let a = INTERNER.intern_str("A");
        let b = INTERNER.intern_str("B");
        let c = INTERNER.intern_str("C");

        assert!(!builder.cache.all::<compile::Std>().is_empty());
        assert_eq!(
            first(builder.cache.all::<compile::Assemble>()),
            &[
                compile::Assemble {
                    target_compiler: Compiler { host: a, stage: 0 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: a, stage: 1 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: b, stage: 1 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: a, stage: 2 },
                },
                compile::Assemble {
                    target_compiler: Compiler { host: b, stage: 2 },
                },
            ]
        );
        assert_eq!(
            first(builder.cache.all::<compile::Rustc>()),
            &[
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 0 },
                    target: a,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 1 },
                    target: a,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 0 },
                    target: b,
                },
                compile::Rustc {
                    compiler: Compiler { host: a, stage: 1 },
                    target: b,
                },
            ]
        );

        assert_eq!(
            first(builder.cache.all::<compile::Test>()),
            &[
                compile::Test {
                    compiler: Compiler { host: a, stage: 0 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 1 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: b, stage: 2 },
                    target: a,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 0 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 1 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: b, stage: 2 },
                    target: b,
                },
                compile::Test {
                    compiler: Compiler { host: a, stage: 2 },
                    target: c,
                },
                compile::Test {
                    compiler: Compiler { host: b, stage: 2 },
                    target: c,
                },
            ]
        );
    }

    #[test]
    fn test_with_no_doc_stage0() {
        let mut config = configure(&[], &[]);
        config.stage = Some(0);
        config.cmd = Subcommand::Test {
            paths: vec!["src/libstd".into()],
            test_args: vec![],
            rustc_args: vec![],
            fail_fast: true,
            doc_tests: DocTests::No,
            bless: false,
            compare_mode: None,
        };

        let build = Build::new(config);
        let mut builder = Builder::new(&build);

        let host = INTERNER.intern_str("A");

        builder.run_step_descriptions(
            &[StepDescription::from::<test::Crate>()],
            &["src/libstd".into()],
        );

        // Ensure we don't build any compiler artifacts.
        assert!(!builder.cache.contains::<compile::Rustc>());
        assert_eq!(
            first(builder.cache.all::<test::Crate>()),
            &[test::Crate {
                compiler: Compiler { host, stage: 0 },
                target: host,
                mode: Mode::Std,
                test_kind: test::TestKind::Test,
                krate: INTERNER.intern_str("std"),
            },]
        );
    }

    #[test]
    fn test_exclude() {
        let mut config = configure(&[], &[]);
        config.exclude = vec![
            "src/test/run-pass".into(),
            "src/tools/tidy".into(),
        ];
        config.cmd = Subcommand::Test {
            paths: Vec::new(),
            test_args: Vec::new(),
            rustc_args: Vec::new(),
            fail_fast: true,
            doc_tests: DocTests::No,
            bless: false,
            compare_mode: None,
        };

        let build = Build::new(config);
        let builder = Builder::new(&build);
        builder.run_step_descriptions(&Builder::get_step_descriptions(Kind::Test), &[]);

        // Ensure we have really excluded run-pass & tidy
        assert!(!builder.cache.contains::<test::RunPass>());
        assert!(!builder.cache.contains::<test::Tidy>());

        // Ensure other tests are not affected.
        assert!(builder.cache.contains::<test::RunPassFullDeps>());
        assert!(builder.cache.contains::<test::RustdocUi>());
    }
}
