use syntax_pos::symbol::Symbol;
use back::write::create_target_machine;
use llvm;
use rustc::session::Session;
use rustc::session::config::PrintRequest;
use libc::c_int;
use std::ffi::CString;
use syntax::feature_gate::UnstableFeatures;

use std::str;
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static POISONED: AtomicBool = AtomicBool::new(false);
static INIT: Once = Once::new();

pub(crate) fn init(sess: &Session) {
    unsafe {
        // Before we touch LLVM, make sure that multithreading is enabled.
        INIT.call_once(|| {
            if llvm::LLVMStartMultithreaded() != 1 {
                // use an extra bool to make sure that all future usage of LLVM
                // cannot proceed despite the Once not running more than once.
                POISONED.store(true, Ordering::SeqCst);
            }

            configure_llvm(sess);
        });

        if POISONED.load(Ordering::SeqCst) {
            bug!("couldn't enable multi-threaded LLVM");
        }
    }
}

fn require_inited() {
    INIT.call_once(|| bug!("llvm is not initialized"));
    if POISONED.load(Ordering::SeqCst) {
        bug!("couldn't enable multi-threaded LLVM");
    }
}

unsafe fn configure_llvm(sess: &Session) {
    let n_args = sess.opts.cg.llvm_args.len();
    let mut llvm_c_strs = Vec::with_capacity(n_args + 1);
    let mut llvm_args = Vec::with_capacity(n_args + 1);

    llvm::LLVMRustInstallFatalErrorHandler();

    {
        let mut add = |arg: &str| {
            let s = CString::new(arg).unwrap();
            llvm_args.push(s.as_ptr());
            llvm_c_strs.push(s);
        };
        add("rustc"); // fake program name
        if sess.time_llvm_passes() { add("-time-passes"); }
        if sess.print_llvm_passes() { add("-debug-pass=Structure"); }
        if sess.opts.debugging_opts.disable_instrumentation_preinliner {
            add("-disable-preinline");
        }
        if llvm::LLVMRustIsRustLLVM() {
            add("-mergefunc-use-aliases");
        }

        // HACK(eddyb) LLVM inserts `llvm.assume` calls to preserve align attributes
        // during inlining. Unfortunately these may block other optimizations.
        add("-preserve-alignment-assumptions-during-inlining=false");

        for arg in &sess.opts.cg.llvm_args {
            add(&(*arg));
        }
    }

    llvm::LLVMInitializePasses();

    ::rustc_llvm::initialize_available_targets();

    llvm::LLVMRustSetLLVMOptions(llvm_args.len() as c_int,
                                 llvm_args.as_ptr());
}

// WARNING: the features after applying `to_llvm_feature` must be known
// to LLVM or the feature detection code will walk past the end of the feature
// array, leading to crashes.

const ARM_WHITELIST: &[(&str, Option<&str>)] = &[
    ("aclass", Some("arm_target_feature")),
    ("mclass", Some("arm_target_feature")),
    ("rclass", Some("arm_target_feature")),
    ("dsp", Some("arm_target_feature")),
    ("neon", Some("arm_target_feature")),
    ("v5te", Some("arm_target_feature")),
    ("v6k", Some("arm_target_feature")),
    ("v6t2", Some("arm_target_feature")),
    ("v7", Some("arm_target_feature")),
    ("vfp2", Some("arm_target_feature")),
    ("vfp3", Some("arm_target_feature")),
    ("vfp4", Some("arm_target_feature")),
];

const AARCH64_WHITELIST: &[(&str, Option<&str>)] = &[
    ("fp", Some("aarch64_target_feature")),
    ("neon", Some("aarch64_target_feature")),
    ("sve", Some("aarch64_target_feature")),
    ("crc", Some("aarch64_target_feature")),
    ("crypto", Some("aarch64_target_feature")),
    ("ras", Some("aarch64_target_feature")),
    ("lse", Some("aarch64_target_feature")),
    ("rdm", Some("aarch64_target_feature")),
    ("fp16", Some("aarch64_target_feature")),
    ("rcpc", Some("aarch64_target_feature")),
    ("dotprod", Some("aarch64_target_feature")),
    ("v8.1a", Some("aarch64_target_feature")),
    ("v8.2a", Some("aarch64_target_feature")),
    ("v8.3a", Some("aarch64_target_feature")),
];

const X86_WHITELIST: &[(&str, Option<&str>)] = &[
    ("adx", Some("adx_target_feature")),
    ("aes", None),
    ("avx", None),
    ("avx2", None),
    ("avx512bw", Some("avx512_target_feature")),
    ("avx512cd", Some("avx512_target_feature")),
    ("avx512dq", Some("avx512_target_feature")),
    ("avx512er", Some("avx512_target_feature")),
    ("avx512f", Some("avx512_target_feature")),
    ("avx512ifma", Some("avx512_target_feature")),
    ("avx512pf", Some("avx512_target_feature")),
    ("avx512vbmi", Some("avx512_target_feature")),
    ("avx512vl", Some("avx512_target_feature")),
    ("avx512vpopcntdq", Some("avx512_target_feature")),
    ("bmi1", None),
    ("bmi2", None),
    ("cmpxchg16b", Some("cmpxchg16b_target_feature")),
    ("fma", None),
    ("fxsr", None),
    ("lzcnt", None),
    ("mmx", Some("mmx_target_feature")),
    ("pclmulqdq", None),
    ("popcnt", None),
    ("rdrand", None),
    ("rdseed", None),
    ("sha", None),
    ("sse", None),
    ("sse2", None),
    ("sse3", None),
    ("sse4.1", None),
    ("sse4.2", None),
    ("sse4a", Some("sse4a_target_feature")),
    ("ssse3", None),
    ("tbm", Some("tbm_target_feature")),
    ("xsave", None),
    ("xsavec", None),
    ("xsaveopt", None),
    ("xsaves", None),
];

const HEXAGON_WHITELIST: &[(&str, Option<&str>)] = &[
    ("hvx", Some("hexagon_target_feature")),
    ("hvx-double", Some("hexagon_target_feature")),
];

const POWERPC_WHITELIST: &[(&str, Option<&str>)] = &[
    ("altivec", Some("powerpc_target_feature")),
    ("power8-altivec", Some("powerpc_target_feature")),
    ("power9-altivec", Some("powerpc_target_feature")),
    ("power8-vector", Some("powerpc_target_feature")),
    ("power9-vector", Some("powerpc_target_feature")),
    ("vsx", Some("powerpc_target_feature")),
];

const MIPS_WHITELIST: &[(&str, Option<&str>)] = &[
    ("fp64", Some("mips_target_feature")),
    ("msa", Some("mips_target_feature")),
];

const WASM_WHITELIST: &[(&str, Option<&str>)] = &[
    ("simd128", Some("wasm_target_feature")),
    ("atomics", Some("wasm_target_feature")),
];

/// When rustdoc is running, provide a list of all known features so that all their respective
/// primitives may be documented.
///
/// IMPORTANT: If you're adding another whitelist to the above lists, make sure to add it to this
/// iterator!
pub fn all_known_features() -> impl Iterator<Item=(&'static str, Option<&'static str>)> {
    ARM_WHITELIST.iter().cloned()
        .chain(AARCH64_WHITELIST.iter().cloned())
        .chain(X86_WHITELIST.iter().cloned())
        .chain(HEXAGON_WHITELIST.iter().cloned())
        .chain(POWERPC_WHITELIST.iter().cloned())
        .chain(MIPS_WHITELIST.iter().cloned())
        .chain(WASM_WHITELIST.iter().cloned())
}

pub fn to_llvm_feature<'a>(sess: &Session, s: &'a str) -> &'a str {
    let arch = if sess.target.target.arch == "x86_64" {
        "x86"
    } else {
        &*sess.target.target.arch
    };
    match (arch, s) {
        ("x86", "pclmulqdq") => "pclmul",
        ("x86", "rdrand") => "rdrnd",
        ("x86", "bmi1") => "bmi",
        ("x86", "cmpxchg16b") => "cx16",
        ("aarch64", "fp") => "fp-armv8",
        ("aarch64", "fp16") => "fullfp16",
        (_, s) => s,
    }
}

pub fn target_features(sess: &Session) -> Vec<Symbol> {
    let target_machine = create_target_machine(sess, true);
    target_feature_whitelist(sess)
        .iter()
        .filter_map(|&(feature, gate)| {
            if UnstableFeatures::from_environment().is_nightly_build() || gate.is_none() {
                Some(feature)
            } else {
                None
            }
        })
        .filter(|feature| {
            let llvm_feature = to_llvm_feature(sess, feature);
            let cstr = CString::new(llvm_feature).unwrap();
            unsafe { llvm::LLVMRustHasFeature(target_machine, cstr.as_ptr()) }
        })
        .map(|feature| Symbol::intern(feature)).collect()
}

pub fn target_feature_whitelist(sess: &Session)
    -> &'static [(&'static str, Option<&'static str>)]
{
    match &*sess.target.target.arch {
        "arm" => ARM_WHITELIST,
        "aarch64" => AARCH64_WHITELIST,
        "x86" | "x86_64" => X86_WHITELIST,
        "hexagon" => HEXAGON_WHITELIST,
        "mips" | "mips64" => MIPS_WHITELIST,
        "powerpc" | "powerpc64" => POWERPC_WHITELIST,
        // wasm32 on emscripten does not support these target features
        "wasm32" if !sess.target.target.options.is_like_emscripten => WASM_WHITELIST,
        _ => &[],
    }
}

pub fn print_version() {
    // Can be called without initializing LLVM
    unsafe {
        println!("LLVM version: {}.{}",
                 llvm::LLVMRustVersionMajor(), llvm::LLVMRustVersionMinor());
    }
}

pub fn get_major_version() -> u32 {
    unsafe { llvm::LLVMRustVersionMajor() }
}

pub fn print_passes() {
    // Can be called without initializing LLVM
    unsafe { llvm::LLVMRustPrintPasses(); }
}

pub(crate) fn print(req: PrintRequest, sess: &Session) {
    require_inited();
    let tm = create_target_machine(sess, true);
    unsafe {
        match req {
            PrintRequest::TargetCPUs => llvm::LLVMRustPrintTargetCPUs(tm),
            PrintRequest::TargetFeatures => llvm::LLVMRustPrintTargetFeatures(tm),
            _ => bug!("rustc_codegen_llvm can't handle print request: {:?}", req),
        }
    }
}

pub fn target_cpu(sess: &Session) -> &str {
    let name = match sess.opts.cg.target_cpu {
        Some(ref s) => &**s,
        None => &*sess.target.target.options.cpu
    };
    if name != "native" {
        return name
    }

    unsafe {
        let mut len = 0;
        let ptr = llvm::LLVMRustGetHostCPUName(&mut len);
        str::from_utf8(slice::from_raw_parts(ptr as *const u8, len)).unwrap()
    }
}
