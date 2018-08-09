// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub use self::code_stats::{CodeStats, DataTypeKind, FieldInfo};
pub use self::code_stats::{SizeKind, TypeSizeInfo, VariantInfo};

use hir::def_id::CrateNum;
use ich::Fingerprint;

use ich;
use lint;
use lint::builtin::BuiltinLintDiagnostics;
use middle::allocator::AllocatorKind;
use middle::dependency_format;
use session::search_paths::PathKind;
use session::config::{OutputType};
use ty::tls;
use util::nodemap::{FxHashMap, FxHashSet};
use util::common::{duration_to_secs_str, ErrorReported};
use util::common::ProfileQueriesMsg;

use rustc_data_structures::sync::{self, Lrc, Lock, LockCell, OneThread, Once, RwLock};

use syntax::ast::NodeId;
use errors::{self, DiagnosticBuilder, DiagnosticId};
use errors::emitter::{Emitter, EmitterWriter};
use syntax::edition::Edition;
use syntax::json::JsonEmitter;
use syntax::feature_gate;
use syntax::symbol::Symbol;
use syntax::parse;
use syntax::parse::ParseSess;
use syntax::{ast, codemap};
use syntax::feature_gate::AttributeType;
use syntax_pos::{MultiSpan, Span};

use rustc_target::spec::{LinkerFlavor, PanicStrategy};
use rustc_target::spec::{Target, TargetTriple};
use rustc_data_structures::flock;
use jobserver::Client;

use std;
use std::cell::{self, Cell, RefCell};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::mpsc;
use std::sync::atomic::{AtomicUsize, Ordering};

mod code_stats;
pub mod config;
pub mod filesearch;
pub mod search_paths;

/// Represents the data associated with a compilation
/// session for a single crate.
pub struct Session {
    pub target: config::Config,
    pub host: Target,
    pub opts: config::Options,
    pub parse_sess: ParseSess,
    /// For a library crate, this is always none
    pub entry_fn: Once<Option<(NodeId, Span, config::EntryFnType)>>,
    pub plugin_registrar_fn: Once<Option<ast::NodeId>>,
    pub derive_registrar_fn: Once<Option<ast::NodeId>>,
    pub default_sysroot: Option<PathBuf>,
    /// The name of the root source file of the crate, in the local file system.
    /// `None` means that there is no source file.
    pub local_crate_source_file: Option<PathBuf>,
    /// The directory the compiler has been executed in plus a flag indicating
    /// if the value stored here has been affected by path remapping.
    pub working_dir: (PathBuf, bool),

    // FIXME: lint_store and buffered_lints are not thread-safe,
    // but are only used in a single thread
    pub lint_store: RwLock<lint::LintStore>,
    pub buffered_lints: Lock<Option<lint::LintBuffer>>,

    /// Set of (DiagnosticId, Option<Span>, message) tuples tracking
    /// (sub)diagnostics that have been set once, but should not be set again,
    /// in order to avoid redundantly verbose output (Issue #24690, #44953).
    pub one_time_diagnostics: Lock<FxHashSet<(DiagnosticMessageId, Option<Span>, String)>>,
    pub plugin_llvm_passes: OneThread<RefCell<Vec<String>>>,
    pub plugin_attributes: OneThread<RefCell<Vec<(String, AttributeType)>>>,
    pub crate_types: Once<Vec<config::CrateType>>,
    pub dependency_formats: Once<dependency_format::Dependencies>,
    /// The crate_disambiguator is constructed out of all the `-C metadata`
    /// arguments passed to the compiler. Its value together with the crate-name
    /// forms a unique global identifier for the crate. It is used to allow
    /// multiple crates with the same name to coexist. See the
    /// rustc_codegen_llvm::back::symbol_names module for more information.
    pub crate_disambiguator: Once<CrateDisambiguator>,

    features: Once<feature_gate::Features>,

    /// The maximum recursion limit for potentially infinitely recursive
    /// operations such as auto-dereference and monomorphization.
    pub recursion_limit: Once<usize>,

    /// The maximum length of types during monomorphization.
    pub type_length_limit: Once<usize>,

    /// The maximum number of stackframes allowed in const eval
    pub const_eval_stack_frame_limit: usize,

    /// The metadata::creader module may inject an allocator/panic_runtime
    /// dependency if it didn't already find one, and this tracks what was
    /// injected.
    pub injected_allocator: Once<Option<CrateNum>>,
    pub allocator_kind: Once<Option<AllocatorKind>>,
    pub injected_panic_runtime: Once<Option<CrateNum>>,

    /// Map from imported macro spans (which consist of
    /// the localized span for the macro body) to the
    /// macro name and definition span in the source crate.
    pub imported_macro_spans: OneThread<RefCell<HashMap<Span, (String, Span)>>>,

    incr_comp_session: OneThread<RefCell<IncrCompSession>>,

    /// A cache of attributes ignored by StableHashingContext
    pub ignored_attr_names: FxHashSet<Symbol>,

    /// Used by -Z profile-queries in util::common
    pub profile_channel: Lock<Option<mpsc::Sender<ProfileQueriesMsg>>>,

    /// Some measurements that are being gathered during compilation.
    pub perf_stats: PerfStats,

    /// Data about code being compiled, gathered during compilation.
    pub code_stats: Lock<CodeStats>,

    next_node_id: OneThread<Cell<ast::NodeId>>,

    /// If -zfuel=crate=n is specified, Some(crate).
    optimization_fuel_crate: Option<String>,
    /// If -zfuel=crate=n is specified, initially set to n. Otherwise 0.
    optimization_fuel_limit: LockCell<u64>,
    /// We're rejecting all further optimizations.
    out_of_fuel: LockCell<bool>,

    // The next two are public because the driver needs to read them.
    /// If -zprint-fuel=crate, Some(crate).
    pub print_fuel_crate: Option<String>,
    /// Always set to zero and incremented so that we can print fuel expended by a crate.
    pub print_fuel: LockCell<u64>,

    /// Loaded up early on in the initialization of this `Session` to avoid
    /// false positives about a job server in our environment.
    pub jobserver: Client,

    /// Metadata about the allocators for the current crate being compiled
    pub has_global_allocator: Once<bool>,

    /// Cap lint level specified by a driver specifically.
    pub driver_lint_caps: FxHashMap<lint::LintId, lint::Level>,
}

pub struct PerfStats {
    /// The accumulated time spent on computing symbol hashes
    pub symbol_hash_time: Lock<Duration>,
    /// The accumulated time spent decoding def path tables from metadata
    pub decode_def_path_tables_time: Lock<Duration>,
    /// Total number of values canonicalized queries constructed.
    pub queries_canonicalized: AtomicUsize,
    /// Number of times this query is invoked.
    pub normalize_ty_after_erasing_regions: AtomicUsize,
    /// Number of times this query is invoked.
    pub normalize_projection_ty: AtomicUsize,
}

/// Enum to support dispatch of one-time diagnostics (in Session.diag_once)
enum DiagnosticBuilderMethod {
    Note,
    SpanNote,
    SpanSuggestion(String), // suggestion
                            // add more variants as needed to support one-time diagnostics
}

/// Diagnostic message ID—used by `Session.one_time_diagnostics` to avoid
/// emitting the same message more than once
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticMessageId {
    ErrorId(u16), // EXXXX error code as integer
    LintId(lint::LintId),
    StabilityId(u32), // issue number
}

impl From<&'static lint::Lint> for DiagnosticMessageId {
    fn from(lint: &'static lint::Lint) -> Self {
        DiagnosticMessageId::LintId(lint::LintId::of(lint))
    }
}

impl Session {
    pub fn local_crate_disambiguator(&self) -> CrateDisambiguator {
        *self.crate_disambiguator.get()
    }

    pub fn struct_span_warn<'a, S: Into<MultiSpan>>(
        &'a self,
        sp: S,
        msg: &str,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_span_warn(sp, msg)
    }
    pub fn struct_span_warn_with_code<'a, S: Into<MultiSpan>>(
        &'a self,
        sp: S,
        msg: &str,
        code: DiagnosticId,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_span_warn_with_code(sp, msg, code)
    }
    pub fn struct_warn<'a>(&'a self, msg: &str) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_warn(msg)
    }
    pub fn struct_span_err<'a, S: Into<MultiSpan>>(
        &'a self,
        sp: S,
        msg: &str,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_span_err(sp, msg)
    }
    pub fn struct_span_err_with_code<'a, S: Into<MultiSpan>>(
        &'a self,
        sp: S,
        msg: &str,
        code: DiagnosticId,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_span_err_with_code(sp, msg, code)
    }
    // FIXME: This method should be removed (every error should have an associated error code).
    pub fn struct_err<'a>(&'a self, msg: &str) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_err(msg)
    }
    pub fn struct_err_with_code<'a>(
        &'a self,
        msg: &str,
        code: DiagnosticId,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_err_with_code(msg, code)
    }
    pub fn struct_span_fatal<'a, S: Into<MultiSpan>>(
        &'a self,
        sp: S,
        msg: &str,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_span_fatal(sp, msg)
    }
    pub fn struct_span_fatal_with_code<'a, S: Into<MultiSpan>>(
        &'a self,
        sp: S,
        msg: &str,
        code: DiagnosticId,
    ) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_span_fatal_with_code(sp, msg, code)
    }
    pub fn struct_fatal<'a>(&'a self, msg: &str) -> DiagnosticBuilder<'a> {
        self.diagnostic().struct_fatal(msg)
    }

    pub fn span_fatal<S: Into<MultiSpan>>(&self, sp: S, msg: &str) -> ! {
        self.diagnostic().span_fatal(sp, msg).raise()
    }
    pub fn span_fatal_with_code<S: Into<MultiSpan>>(
        &self,
        sp: S,
        msg: &str,
        code: DiagnosticId,
    ) -> ! {
        self.diagnostic()
            .span_fatal_with_code(sp, msg, code)
            .raise()
    }
    pub fn fatal(&self, msg: &str) -> ! {
        self.diagnostic().fatal(msg).raise()
    }
    pub fn span_err_or_warn<S: Into<MultiSpan>>(&self, is_warning: bool, sp: S, msg: &str) {
        if is_warning {
            self.span_warn(sp, msg);
        } else {
            self.span_err(sp, msg);
        }
    }
    pub fn span_err<S: Into<MultiSpan>>(&self, sp: S, msg: &str) {
        self.diagnostic().span_err(sp, msg)
    }
    pub fn span_err_with_code<S: Into<MultiSpan>>(&self, sp: S, msg: &str, code: DiagnosticId) {
        self.diagnostic().span_err_with_code(sp, &msg, code)
    }
    pub fn err(&self, msg: &str) {
        self.diagnostic().err(msg)
    }
    pub fn err_count(&self) -> usize {
        self.diagnostic().err_count()
    }
    pub fn has_errors(&self) -> bool {
        self.diagnostic().has_errors()
    }
    pub fn abort_if_errors(&self) {
        self.diagnostic().abort_if_errors();
    }
    pub fn compile_status(&self) -> Result<(), CompileIncomplete> {
        compile_result_from_err_count(self.err_count())
    }
    pub fn track_errors<F, T>(&self, f: F) -> Result<T, ErrorReported>
    where
        F: FnOnce() -> T,
    {
        let old_count = self.err_count();
        let result = f();
        let errors = self.err_count() - old_count;
        if errors == 0 {
            Ok(result)
        } else {
            Err(ErrorReported)
        }
    }
    pub fn span_warn<S: Into<MultiSpan>>(&self, sp: S, msg: &str) {
        self.diagnostic().span_warn(sp, msg)
    }
    pub fn span_warn_with_code<S: Into<MultiSpan>>(&self, sp: S, msg: &str, code: DiagnosticId) {
        self.diagnostic().span_warn_with_code(sp, msg, code)
    }
    pub fn warn(&self, msg: &str) {
        self.diagnostic().warn(msg)
    }
    pub fn opt_span_warn<S: Into<MultiSpan>>(&self, opt_sp: Option<S>, msg: &str) {
        match opt_sp {
            Some(sp) => self.span_warn(sp, msg),
            None => self.warn(msg),
        }
    }
    /// Delay a span_bug() call until abort_if_errors()
    pub fn delay_span_bug<S: Into<MultiSpan>>(&self, sp: S, msg: &str) {
        self.diagnostic().delay_span_bug(sp, msg)
    }
    pub fn note_without_error(&self, msg: &str) {
        self.diagnostic().note_without_error(msg)
    }
    pub fn span_note_without_error<S: Into<MultiSpan>>(&self, sp: S, msg: &str) {
        self.diagnostic().span_note_without_error(sp, msg)
    }
    pub fn span_unimpl<S: Into<MultiSpan>>(&self, sp: S, msg: &str) -> ! {
        self.diagnostic().span_unimpl(sp, msg)
    }
    pub fn unimpl(&self, msg: &str) -> ! {
        self.diagnostic().unimpl(msg)
    }

    pub fn buffer_lint<S: Into<MultiSpan>>(
        &self,
        lint: &'static lint::Lint,
        id: ast::NodeId,
        sp: S,
        msg: &str,
    ) {
        match *self.buffered_lints.borrow_mut() {
            Some(ref mut buffer) => {
                buffer.add_lint(lint, id, sp.into(), msg, BuiltinLintDiagnostics::Normal)
            }
            None => bug!("can't buffer lints after HIR lowering"),
        }
    }

    pub fn buffer_lint_with_diagnostic<S: Into<MultiSpan>>(
        &self,
        lint: &'static lint::Lint,
        id: ast::NodeId,
        sp: S,
        msg: &str,
        diagnostic: BuiltinLintDiagnostics,
    ) {
        match *self.buffered_lints.borrow_mut() {
            Some(ref mut buffer) => buffer.add_lint(lint, id, sp.into(), msg, diagnostic),
            None => bug!("can't buffer lints after HIR lowering"),
        }
    }

    pub fn reserve_node_ids(&self, count: usize) -> ast::NodeId {
        let id = self.next_node_id.get();

        match id.as_usize().checked_add(count) {
            Some(next) => {
                self.next_node_id.set(ast::NodeId::new(next));
            }
            None => bug!("Input too large, ran out of node ids!"),
        }

        id
    }
    pub fn next_node_id(&self) -> NodeId {
        self.reserve_node_ids(1)
    }
    pub fn diagnostic<'a>(&'a self) -> &'a errors::Handler {
        &self.parse_sess.span_diagnostic
    }

    /// Analogous to calling methods on the given `DiagnosticBuilder`, but
    /// deduplicates on lint ID, span (if any), and message for this `Session`
    fn diag_once<'a, 'b>(
        &'a self,
        diag_builder: &'b mut DiagnosticBuilder<'a>,
        method: DiagnosticBuilderMethod,
        msg_id: DiagnosticMessageId,
        message: &str,
        span_maybe: Option<Span>,
    ) {
        let id_span_message = (msg_id, span_maybe, message.to_owned());
        let fresh = self.one_time_diagnostics
            .borrow_mut()
            .insert(id_span_message);
        if fresh {
            match method {
                DiagnosticBuilderMethod::Note => {
                    diag_builder.note(message);
                }
                DiagnosticBuilderMethod::SpanNote => {
                    let span = span_maybe.expect("span_note needs a span");
                    diag_builder.span_note(span, message);
                }
                DiagnosticBuilderMethod::SpanSuggestion(suggestion) => {
                    let span = span_maybe.expect("span_suggestion needs a span");
                    diag_builder.span_suggestion(span, message, suggestion);
                }
            }
        }
    }

    pub fn diag_span_note_once<'a, 'b>(
        &'a self,
        diag_builder: &'b mut DiagnosticBuilder<'a>,
        msg_id: DiagnosticMessageId,
        span: Span,
        message: &str,
    ) {
        self.diag_once(
            diag_builder,
            DiagnosticBuilderMethod::SpanNote,
            msg_id,
            message,
            Some(span),
        );
    }

    pub fn diag_note_once<'a, 'b>(
        &'a self,
        diag_builder: &'b mut DiagnosticBuilder<'a>,
        msg_id: DiagnosticMessageId,
        message: &str,
    ) {
        self.diag_once(
            diag_builder,
            DiagnosticBuilderMethod::Note,
            msg_id,
            message,
            None,
        );
    }

    pub fn diag_span_suggestion_once<'a, 'b>(
        &'a self,
        diag_builder: &'b mut DiagnosticBuilder<'a>,
        msg_id: DiagnosticMessageId,
        span: Span,
        message: &str,
        suggestion: String,
    ) {
        self.diag_once(
            diag_builder,
            DiagnosticBuilderMethod::SpanSuggestion(suggestion),
            msg_id,
            message,
            Some(span),
        );
    }

    pub fn codemap<'a>(&'a self) -> &'a codemap::CodeMap {
        self.parse_sess.codemap()
    }
    pub fn verbose(&self) -> bool {
        self.opts.debugging_opts.verbose
    }
    pub fn time_passes(&self) -> bool {
        self.opts.debugging_opts.time_passes
    }
    pub fn profile_queries(&self) -> bool {
        self.opts.debugging_opts.profile_queries
            || self.opts.debugging_opts.profile_queries_and_keys
    }
    pub fn profile_queries_and_keys(&self) -> bool {
        self.opts.debugging_opts.profile_queries_and_keys
    }
    pub fn count_llvm_insns(&self) -> bool {
        self.opts.debugging_opts.count_llvm_insns
    }
    pub fn time_llvm_passes(&self) -> bool {
        self.opts.debugging_opts.time_llvm_passes
    }
    pub fn codegen_stats(&self) -> bool {
        self.opts.debugging_opts.codegen_stats
    }
    pub fn meta_stats(&self) -> bool {
        self.opts.debugging_opts.meta_stats
    }
    pub fn asm_comments(&self) -> bool {
        self.opts.debugging_opts.asm_comments
    }
    pub fn no_verify(&self) -> bool {
        self.opts.debugging_opts.no_verify
    }
    pub fn borrowck_stats(&self) -> bool {
        self.opts.debugging_opts.borrowck_stats
    }
    pub fn print_llvm_passes(&self) -> bool {
        self.opts.debugging_opts.print_llvm_passes
    }

    /// Get the features enabled for the current compilation session.
    /// DO NOT USE THIS METHOD if there is a TyCtxt available, as it circumvents
    /// dependency tracking. Use tcx.features() instead.
    #[inline]
    pub fn features_untracked(&self) -> &feature_gate::Features {
        self.features.get()
    }

    pub fn init_features(&self, features: feature_gate::Features) {
        self.features.set(features);
    }

    /// Calculates the flavor of LTO to use for this compilation.
    pub fn lto(&self) -> config::Lto {
        // If our target has codegen requirements ignore the command line
        if self.target.target.options.requires_lto {
            return config::Lto::Fat;
        }

        // If the user specified something, return that. If they only said `-C
        // lto` and we've for whatever reason forced off ThinLTO via the CLI,
        // then ensure we can't use a ThinLTO.
        match self.opts.cg.lto {
            config::Lto::No => {}
            config::Lto::Yes if self.opts.cli_forced_thinlto_off => return config::Lto::Fat,
            other => return other,
        }

        // Ok at this point the target doesn't require anything and the user
        // hasn't asked for anything. Our next decision is whether or not
        // we enable "auto" ThinLTO where we use multiple codegen units and
        // then do ThinLTO over those codegen units. The logic below will
        // either return `No` or `ThinLocal`.

        // If processing command line options determined that we're incompatible
        // with ThinLTO (e.g. `-C lto --emit llvm-ir`) then return that option.
        if self.opts.cli_forced_thinlto_off {
            return config::Lto::No;
        }

        // If `-Z thinlto` specified process that, but note that this is mostly
        // a deprecated option now that `-C lto=thin` exists.
        if let Some(enabled) = self.opts.debugging_opts.thinlto {
            if enabled {
                return config::Lto::ThinLocal;
            } else {
                return config::Lto::No;
            }
        }

        // If there's only one codegen unit and LTO isn't enabled then there's
        // no need for ThinLTO so just return false.
        if self.codegen_units() == 1 {
            return config::Lto::No;
        }

        // Right now ThinLTO isn't compatible with incremental compilation.
        if self.opts.incremental.is_some() {
            return config::Lto::No;
        }

        // Now we're in "defaults" territory. By default we enable ThinLTO for
        // optimized compiles (anything greater than O0).
        match self.opts.optimize {
            config::OptLevel::No => config::Lto::No,
            _ => config::Lto::ThinLocal,
        }
    }

    /// Returns the panic strategy for this compile session. If the user explicitly selected one
    /// using '-C panic', use that, otherwise use the panic strategy defined by the target.
    pub fn panic_strategy(&self) -> PanicStrategy {
        self.opts
            .cg
            .panic
            .unwrap_or(self.target.target.options.panic_strategy)
    }
    pub fn linker_flavor(&self) -> LinkerFlavor {
        self.opts
            .debugging_opts
            .linker_flavor
            .unwrap_or(self.target.target.linker_flavor)
    }

    pub fn fewer_names(&self) -> bool {
        let more_names = self.opts
            .output_types
            .contains_key(&OutputType::LlvmAssembly)
            || self.opts.output_types.contains_key(&OutputType::Bitcode);
        self.opts.debugging_opts.fewer_names || !more_names
    }

    pub fn no_landing_pads(&self) -> bool {
        self.opts.debugging_opts.no_landing_pads || self.panic_strategy() == PanicStrategy::Abort
    }
    pub fn unstable_options(&self) -> bool {
        self.opts.debugging_opts.unstable_options
    }
    pub fn nonzeroing_move_hints(&self) -> bool {
        self.opts.debugging_opts.enable_nonzeroing_move_hints
    }
    pub fn overflow_checks(&self) -> bool {
        self.opts
            .cg
            .overflow_checks
            .or(self.opts.debugging_opts.force_overflow_checks)
            .unwrap_or(self.opts.debug_assertions)
    }

    pub fn crt_static(&self) -> bool {
        // If the target does not opt in to crt-static support, use its default.
        if self.target.target.options.crt_static_respected {
            self.crt_static_feature()
        } else {
            self.target.target.options.crt_static_default
        }
    }

    pub fn crt_static_feature(&self) -> bool {
        let requested_features = self.opts.cg.target_feature.split(',');
        let found_negative = requested_features.clone().any(|r| r == "-crt-static");
        let found_positive = requested_features.clone().any(|r| r == "+crt-static");

        // If the target we're compiling for requests a static crt by default,
        // then see if the `-crt-static` feature was passed to disable that.
        // Otherwise if we don't have a static crt by default then see if the
        // `+crt-static` feature was passed.
        if self.target.target.options.crt_static_default {
            !found_negative
        } else {
            found_positive
        }
    }

    pub fn target_cpu(&self) -> &str {
        match self.opts.cg.target_cpu {
            Some(ref s) => &**s,
            None => &*self.target.target.options.cpu
        }
    }

    pub fn must_not_eliminate_frame_pointers(&self) -> bool {
        if let Some(x) = self.opts.cg.force_frame_pointers {
            x
        } else {
            !self.target.target.options.eliminate_frame_pointer
        }
    }

    /// Returns the symbol name for the registrar function,
    /// given the crate Svh and the function DefIndex.
    pub fn generate_plugin_registrar_symbol(&self, disambiguator: CrateDisambiguator) -> String {
        format!(
            "__rustc_plugin_registrar_{}__",
            disambiguator.to_fingerprint().to_hex()
        )
    }

    pub fn generate_derive_registrar_symbol(&self, disambiguator: CrateDisambiguator) -> String {
        format!(
            "__rustc_derive_registrar_{}__",
            disambiguator.to_fingerprint().to_hex()
        )
    }

    pub fn sysroot<'a>(&'a self) -> &'a Path {
        match self.opts.maybe_sysroot {
            Some(ref sysroot) => sysroot,
            None => self.default_sysroot
                .as_ref()
                .expect("missing sysroot and default_sysroot in Session"),
        }
    }
    pub fn target_filesearch(&self, kind: PathKind) -> filesearch::FileSearch {
        filesearch::FileSearch::new(
            self.sysroot(),
            self.opts.target_triple.triple(),
            &self.opts.search_paths,
            kind,
        )
    }
    pub fn host_filesearch(&self, kind: PathKind) -> filesearch::FileSearch {
        filesearch::FileSearch::new(
            self.sysroot(),
            config::host_triple(),
            &self.opts.search_paths,
            kind,
        )
    }

    pub fn set_incr_session_load_dep_graph(&self, load: bool) {
        let mut incr_comp_session = self.incr_comp_session.borrow_mut();

        match *incr_comp_session {
            IncrCompSession::Active {
                ref mut load_dep_graph,
                ..
            } => {
                *load_dep_graph = load;
            }
            _ => {}
        }
    }

    pub fn incr_session_load_dep_graph(&self) -> bool {
        let incr_comp_session = self.incr_comp_session.borrow();
        match *incr_comp_session {
            IncrCompSession::Active { load_dep_graph, .. } => load_dep_graph,
            _ => false,
        }
    }

    pub fn init_incr_comp_session(
        &self,
        session_dir: PathBuf,
        lock_file: flock::Lock,
        load_dep_graph: bool,
    ) {
        let mut incr_comp_session = self.incr_comp_session.borrow_mut();

        if let IncrCompSession::NotInitialized = *incr_comp_session {
        } else {
            bug!(
                "Trying to initialize IncrCompSession `{:?}`",
                *incr_comp_session
            )
        }

        *incr_comp_session = IncrCompSession::Active {
            session_directory: session_dir,
            lock_file,
            load_dep_graph,
        };
    }

    pub fn finalize_incr_comp_session(&self, new_directory_path: PathBuf) {
        let mut incr_comp_session = self.incr_comp_session.borrow_mut();

        if let IncrCompSession::Active { .. } = *incr_comp_session {
        } else {
            bug!(
                "Trying to finalize IncrCompSession `{:?}`",
                *incr_comp_session
            )
        }

        // Note: This will also drop the lock file, thus unlocking the directory
        *incr_comp_session = IncrCompSession::Finalized {
            session_directory: new_directory_path,
        };
    }

    pub fn mark_incr_comp_session_as_invalid(&self) {
        let mut incr_comp_session = self.incr_comp_session.borrow_mut();

        let session_directory = match *incr_comp_session {
            IncrCompSession::Active {
                ref session_directory,
                ..
            } => session_directory.clone(),
            IncrCompSession::InvalidBecauseOfErrors { .. } => return,
            _ => bug!(
                "Trying to invalidate IncrCompSession `{:?}`",
                *incr_comp_session
            ),
        };

        // Note: This will also drop the lock file, thus unlocking the directory
        *incr_comp_session = IncrCompSession::InvalidBecauseOfErrors { session_directory };
    }

    pub fn incr_comp_session_dir(&self) -> cell::Ref<PathBuf> {
        let incr_comp_session = self.incr_comp_session.borrow();
        cell::Ref::map(
            incr_comp_session,
            |incr_comp_session| match *incr_comp_session {
                IncrCompSession::NotInitialized => bug!(
                    "Trying to get session directory from IncrCompSession `{:?}`",
                    *incr_comp_session
                ),
                IncrCompSession::Active {
                    ref session_directory,
                    ..
                }
                | IncrCompSession::Finalized {
                    ref session_directory,
                }
                | IncrCompSession::InvalidBecauseOfErrors {
                    ref session_directory,
                } => session_directory,
            },
        )
    }

    pub fn incr_comp_session_dir_opt(&self) -> Option<cell::Ref<PathBuf>> {
        if self.opts.incremental.is_some() {
            Some(self.incr_comp_session_dir())
        } else {
            None
        }
    }

    pub fn print_perf_stats(&self) {
        println!(
            "Total time spent computing symbol hashes:      {}",
            duration_to_secs_str(*self.perf_stats.symbol_hash_time.lock())
        );
        println!(
            "Total time spent decoding DefPath tables:      {}",
            duration_to_secs_str(*self.perf_stats.decode_def_path_tables_time.lock())
        );
        println!("Total queries canonicalized:                   {}",
                 self.perf_stats.queries_canonicalized.load(Ordering::Relaxed));
        println!("normalize_ty_after_erasing_regions:            {}",
                 self.perf_stats.normalize_ty_after_erasing_regions.load(Ordering::Relaxed));
        println!("normalize_projection_ty:                       {}",
                 self.perf_stats.normalize_projection_ty.load(Ordering::Relaxed));
    }

    /// We want to know if we're allowed to do an optimization for crate foo from -z fuel=foo=n.
    /// This expends fuel if applicable, and records fuel if applicable.
    pub fn consider_optimizing<T: Fn() -> String>(&self, crate_name: &str, msg: T) -> bool {
        let mut ret = true;
        match self.optimization_fuel_crate {
            Some(ref c) if c == crate_name => {
                assert!(self.query_threads() == 1);
                let fuel = self.optimization_fuel_limit.get();
                ret = fuel != 0;
                if fuel == 0 && !self.out_of_fuel.get() {
                    println!("optimization-fuel-exhausted: {}", msg());
                    self.out_of_fuel.set(true);
                } else if fuel > 0 {
                    self.optimization_fuel_limit.set(fuel - 1);
                }
            }
            _ => {}
        }
        match self.print_fuel_crate {
            Some(ref c) if c == crate_name => {
                assert!(self.query_threads() == 1);
                self.print_fuel.set(self.print_fuel.get() + 1);
            }
            _ => {}
        }
        ret
    }

    /// Returns the number of query threads that should be used for this
    /// compilation
    pub fn query_threads_from_opts(opts: &config::Options) -> usize {
        opts.debugging_opts.query_threads.unwrap_or(1)
    }

    /// Returns the number of query threads that should be used for this
    /// compilation
    pub fn query_threads(&self) -> usize {
        Self::query_threads_from_opts(&self.opts)
    }

    /// Returns the number of codegen units that should be used for this
    /// compilation
    pub fn codegen_units(&self) -> usize {
        if let Some(n) = self.opts.cli_forced_codegen_units {
            return n;
        }
        if let Some(n) = self.target.target.options.default_codegen_units {
            return n as usize;
        }

        // Why is 16 codegen units the default all the time?
        //
        // The main reason for enabling multiple codegen units by default is to
        // leverage the ability for the codegen backend to do codegen and
        // optimization in parallel. This allows us, especially for large crates, to
        // make good use of all available resources on the machine once we've
        // hit that stage of compilation. Large crates especially then often
        // take a long time in codegen/optimization and this helps us amortize that
        // cost.
        //
        // Note that a high number here doesn't mean that we'll be spawning a
        // large number of threads in parallel. The backend of rustc contains
        // global rate limiting through the `jobserver` crate so we'll never
        // overload the system with too much work, but rather we'll only be
        // optimizing when we're otherwise cooperating with other instances of
        // rustc.
        //
        // Rather a high number here means that we should be able to keep a lot
        // of idle cpus busy. By ensuring that no codegen unit takes *too* long
        // to build we'll be guaranteed that all cpus will finish pretty closely
        // to one another and we should make relatively optimal use of system
        // resources
        //
        // Note that the main cost of codegen units is that it prevents LLVM
        // from inlining across codegen units. Users in general don't have a lot
        // of control over how codegen units are split up so it's our job in the
        // compiler to ensure that undue performance isn't lost when using
        // codegen units (aka we can't require everyone to slap `#[inline]` on
        // everything).
        //
        // If we're compiling at `-O0` then the number doesn't really matter too
        // much because performance doesn't matter and inlining is ok to lose.
        // In debug mode we just want to try to guarantee that no cpu is stuck
        // doing work that could otherwise be farmed to others.
        //
        // In release mode, however (O1 and above) performance does indeed
        // matter! To recover the loss in performance due to inlining we'll be
        // enabling ThinLTO by default (the function for which is just below).
        // This will ensure that we recover any inlining wins we otherwise lost
        // through codegen unit partitioning.
        //
        // ---
        //
        // Ok that's a lot of words but the basic tl;dr; is that we want a high
        // number here -- but not too high. Additionally we're "safe" to have it
        // always at the same number at all optimization levels.
        //
        // As a result 16 was chosen here! Mostly because it was a power of 2
        // and most benchmarks agreed it was roughly a local optimum. Not very
        // scientific.
        16
    }

    pub fn teach(&self, code: &DiagnosticId) -> bool {
        self.opts.debugging_opts.teach && self.parse_sess.span_diagnostic.must_teach(code)
    }

    /// Are we allowed to use features from the Rust 2018 edition?
    pub fn rust_2018(&self) -> bool {
        self.opts.edition >= Edition::Edition2018
    }

    pub fn edition(&self) -> Edition {
        self.opts.edition
    }
}

pub fn build_session(
    sopts: config::Options,
    local_crate_source_file: Option<PathBuf>,
    registry: errors::registry::Registry,
) -> Session {
    let file_path_mapping = sopts.file_path_mapping();

    build_session_with_codemap(
        sopts,
        local_crate_source_file,
        registry,
        Lrc::new(codemap::CodeMap::new(file_path_mapping)),
        None,
    )
}

pub fn build_session_with_codemap(
    sopts: config::Options,
    local_crate_source_file: Option<PathBuf>,
    registry: errors::registry::Registry,
    codemap: Lrc<codemap::CodeMap>,
    emitter_dest: Option<Box<dyn Write + Send>>,
) -> Session {
    // FIXME: This is not general enough to make the warning lint completely override
    // normal diagnostic warnings, since the warning lint can also be denied and changed
    // later via the source code.
    let warnings_allow = sopts
        .lint_opts
        .iter()
        .filter(|&&(ref key, _)| *key == "warnings")
        .map(|&(_, ref level)| *level == lint::Allow)
        .last()
        .unwrap_or(false);
    let cap_lints_allow = sopts.lint_cap.map_or(false, |cap| cap == lint::Allow);

    let can_emit_warnings = !(warnings_allow || cap_lints_allow);

    let treat_err_as_bug = sopts.debugging_opts.treat_err_as_bug;

    let external_macro_backtrace = sopts.debugging_opts.external_macro_backtrace;

    let emitter: Box<dyn Emitter + sync::Send> =
        match (sopts.error_format, emitter_dest) {
            (config::ErrorOutputType::HumanReadable(color_config), None) => Box::new(
                EmitterWriter::stderr(
                    color_config,
                    Some(codemap.clone()),
                    false,
                    sopts.debugging_opts.teach,
                ).ui_testing(sopts.debugging_opts.ui_testing),
            ),
            (config::ErrorOutputType::HumanReadable(_), Some(dst)) => Box::new(
                EmitterWriter::new(dst, Some(codemap.clone()), false, false)
                    .ui_testing(sopts.debugging_opts.ui_testing),
            ),
            (config::ErrorOutputType::Json(pretty), None) => Box::new(
                JsonEmitter::stderr(
                    Some(registry),
                    codemap.clone(),
                    pretty,
                ).ui_testing(sopts.debugging_opts.ui_testing),
            ),
            (config::ErrorOutputType::Json(pretty), Some(dst)) => Box::new(
                JsonEmitter::new(
                    dst,
                    Some(registry),
                    codemap.clone(),
                    pretty,
                ).ui_testing(sopts.debugging_opts.ui_testing),
            ),
            (config::ErrorOutputType::Short(color_config), None) => Box::new(
                EmitterWriter::stderr(color_config, Some(codemap.clone()), true, false),
            ),
            (config::ErrorOutputType::Short(_), Some(dst)) => {
                Box::new(EmitterWriter::new(dst, Some(codemap.clone()), true, false))
            }
        };

    let diagnostic_handler = errors::Handler::with_emitter_and_flags(
        emitter,
        errors::HandlerFlags {
            can_emit_warnings,
            treat_err_as_bug,
            external_macro_backtrace,
            ..Default::default()
        },
    );

    build_session_(sopts, local_crate_source_file, diagnostic_handler, codemap)
}

pub fn build_session_(
    sopts: config::Options,
    local_crate_source_file: Option<PathBuf>,
    span_diagnostic: errors::Handler,
    codemap: Lrc<codemap::CodeMap>,
) -> Session {
    let host_triple = TargetTriple::from_triple(config::host_triple());
    let host = match Target::search(&host_triple) {
        Ok(t) => t,
        Err(e) => {
            span_diagnostic
                .fatal(&format!("Error loading host specification: {}", e))
                .raise();
        }
    };
    let target_cfg = config::build_target_config(&sopts, &span_diagnostic);

    let p_s = parse::ParseSess::with_span_handler(span_diagnostic, codemap);
    let default_sysroot = match sopts.maybe_sysroot {
        Some(_) => None,
        None => Some(filesearch::get_or_default_sysroot()),
    };

    let file_path_mapping = sopts.file_path_mapping();

    let local_crate_source_file =
        local_crate_source_file.map(|path| file_path_mapping.map_prefix(path).0);

    let optimization_fuel_crate = sopts.debugging_opts.fuel.as_ref().map(|i| i.0.clone());
    let optimization_fuel_limit =
        LockCell::new(sopts.debugging_opts.fuel.as_ref().map(|i| i.1).unwrap_or(0));
    let print_fuel_crate = sopts.debugging_opts.print_fuel.clone();
    let print_fuel = LockCell::new(0);

    let working_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => p_s.span_diagnostic
            .fatal(&format!("Current directory is invalid: {}", e))
            .raise(),
    };
    let working_dir = file_path_mapping.map_prefix(working_dir);

    let sess = Session {
        target: target_cfg,
        host,
        opts: sopts,
        parse_sess: p_s,
        // For a library crate, this is always none
        entry_fn: Once::new(),
        plugin_registrar_fn: Once::new(),
        derive_registrar_fn: Once::new(),
        default_sysroot,
        local_crate_source_file,
        working_dir,
        lint_store: RwLock::new(lint::LintStore::new()),
        buffered_lints: Lock::new(Some(lint::LintBuffer::new())),
        one_time_diagnostics: Lock::new(FxHashSet()),
        plugin_llvm_passes: OneThread::new(RefCell::new(Vec::new())),
        plugin_attributes: OneThread::new(RefCell::new(Vec::new())),
        crate_types: Once::new(),
        dependency_formats: Once::new(),
        crate_disambiguator: Once::new(),
        features: Once::new(),
        recursion_limit: Once::new(),
        type_length_limit: Once::new(),
        const_eval_stack_frame_limit: 100,
        next_node_id: OneThread::new(Cell::new(NodeId::new(1))),
        injected_allocator: Once::new(),
        allocator_kind: Once::new(),
        injected_panic_runtime: Once::new(),
        imported_macro_spans: OneThread::new(RefCell::new(HashMap::new())),
        incr_comp_session: OneThread::new(RefCell::new(IncrCompSession::NotInitialized)),
        ignored_attr_names: ich::compute_ignored_attr_names(),
        profile_channel: Lock::new(None),
        perf_stats: PerfStats {
            symbol_hash_time: Lock::new(Duration::from_secs(0)),
            decode_def_path_tables_time: Lock::new(Duration::from_secs(0)),
            queries_canonicalized: AtomicUsize::new(0),
            normalize_ty_after_erasing_regions: AtomicUsize::new(0),
            normalize_projection_ty: AtomicUsize::new(0),
        },
        code_stats: Lock::new(CodeStats::new()),
        optimization_fuel_crate,
        optimization_fuel_limit,
        print_fuel_crate,
        print_fuel,
        out_of_fuel: LockCell::new(false),
        // Note that this is unsafe because it may misinterpret file descriptors
        // on Unix as jobserver file descriptors. We hopefully execute this near
        // the beginning of the process though to ensure we don't get false
        // positives, or in other words we try to execute this before we open
        // any file descriptors ourselves.
        //
        // Pick a "reasonable maximum" if we don't otherwise have
        // a jobserver in our environment, capping out at 32 so we
        // don't take everything down by hogging the process run queue.
        // The fixed number is used to have deterministic compilation
        // across machines.
        //
        // Also note that we stick this in a global because there could be
        // multiple `Session` instances in this process, and the jobserver is
        // per-process.
        jobserver: unsafe {
            static mut GLOBAL_JOBSERVER: *mut Client = 0 as *mut _;
            static INIT: std::sync::Once = std::sync::ONCE_INIT;
            INIT.call_once(|| {
                let client = Client::from_env().unwrap_or_else(|| {
                    Client::new(32).expect("failed to create jobserver")
                });
                GLOBAL_JOBSERVER = Box::into_raw(Box::new(client));
            });
            (*GLOBAL_JOBSERVER).clone()
        },
        has_global_allocator: Once::new(),
        driver_lint_caps: FxHashMap(),
    };

    sess
}

/// Hash value constructed out of all the `-C metadata` arguments passed to the
/// compiler. Together with the crate-name forms a unique global identifier for
/// the crate.
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Clone, Copy, RustcEncodable, RustcDecodable)]
pub struct CrateDisambiguator(Fingerprint);

impl CrateDisambiguator {
    pub fn to_fingerprint(self) -> Fingerprint {
        self.0
    }
}

impl From<Fingerprint> for CrateDisambiguator {
    fn from(fingerprint: Fingerprint) -> CrateDisambiguator {
        CrateDisambiguator(fingerprint)
    }
}

impl_stable_hash_for!(tuple_struct CrateDisambiguator { fingerprint });

/// Holds data on the current incremental compilation session, if there is one.
#[derive(Debug)]
pub enum IncrCompSession {
    /// This is the state the session will be in until the incr. comp. dir is
    /// needed.
    NotInitialized,
    /// This is the state during which the session directory is private and can
    /// be modified.
    Active {
        session_directory: PathBuf,
        lock_file: flock::Lock,
        load_dep_graph: bool,
    },
    /// This is the state after the session directory has been finalized. In this
    /// state, the contents of the directory must not be modified any more.
    Finalized { session_directory: PathBuf },
    /// This is an error state that is reached when some compilation error has
    /// occurred. It indicates that the contents of the session directory must
    /// not be used, since they might be invalid.
    InvalidBecauseOfErrors { session_directory: PathBuf },
}

pub fn early_error(output: config::ErrorOutputType, msg: &str) -> ! {
    let emitter: Box<dyn Emitter + sync::Send> = match output {
        config::ErrorOutputType::HumanReadable(color_config) => {
            Box::new(EmitterWriter::stderr(color_config, None, false, false))
        }
        config::ErrorOutputType::Json(pretty) => Box::new(JsonEmitter::basic(pretty)),
        config::ErrorOutputType::Short(color_config) => {
            Box::new(EmitterWriter::stderr(color_config, None, true, false))
        }
    };
    let handler = errors::Handler::with_emitter(true, false, emitter);
    handler.emit(&MultiSpan::new(), msg, errors::Level::Fatal);
    errors::FatalError.raise();
}

pub fn early_warn(output: config::ErrorOutputType, msg: &str) {
    let emitter: Box<dyn Emitter + sync::Send> = match output {
        config::ErrorOutputType::HumanReadable(color_config) => {
            Box::new(EmitterWriter::stderr(color_config, None, false, false))
        }
        config::ErrorOutputType::Json(pretty) => Box::new(JsonEmitter::basic(pretty)),
        config::ErrorOutputType::Short(color_config) => {
            Box::new(EmitterWriter::stderr(color_config, None, true, false))
        }
    };
    let handler = errors::Handler::with_emitter(true, false, emitter);
    handler.emit(&MultiSpan::new(), msg, errors::Level::Warning);
}

#[derive(Copy, Clone, Debug)]
pub enum CompileIncomplete {
    Stopped,
    Errored(ErrorReported),
}
impl From<ErrorReported> for CompileIncomplete {
    fn from(err: ErrorReported) -> CompileIncomplete {
        CompileIncomplete::Errored(err)
    }
}
pub type CompileResult = Result<(), CompileIncomplete>;

pub fn compile_result_from_err_count(err_count: usize) -> CompileResult {
    if err_count == 0 {
        Ok(())
    } else {
        Err(CompileIncomplete::Errored(ErrorReported))
    }
}

#[cold]
#[inline(never)]
pub fn bug_fmt(file: &'static str, line: u32, args: fmt::Arguments) -> ! {
    // this wrapper mostly exists so I don't have to write a fully
    // qualified path of None::<Span> inside the bug!() macro definition
    opt_span_bug_fmt(file, line, None::<Span>, args);
}

#[cold]
#[inline(never)]
pub fn span_bug_fmt<S: Into<MultiSpan>>(
    file: &'static str,
    line: u32,
    span: S,
    args: fmt::Arguments,
) -> ! {
    opt_span_bug_fmt(file, line, Some(span), args);
}

fn opt_span_bug_fmt<S: Into<MultiSpan>>(
    file: &'static str,
    line: u32,
    span: Option<S>,
    args: fmt::Arguments,
) -> ! {
    tls::with_opt(move |tcx| {
        let msg = format!("{}:{}: {}", file, line, args);
        match (tcx, span) {
            (Some(tcx), Some(span)) => tcx.sess.diagnostic().span_bug(span, &msg),
            (Some(tcx), None) => tcx.sess.diagnostic().bug(&msg),
            (None, _) => panic!(msg),
        }
    });
    unreachable!();
}
