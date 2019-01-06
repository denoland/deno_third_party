// The wasm32-unknown-unknown target is currently an experimental version of a
// wasm-based target which does *not* use the Emscripten toolchain. Instead
// this toolchain is based purely on LLVM's own toolchain, using LLVM's native
// WebAssembly backend as well as LLD for a native linker.
//
// There's some trickery below on crate types supported and various defaults
// (aka panic=abort by default), but otherwise this is in general a relatively
// standard target.

use super::{LldFlavor, LinkerFlavor, Target, TargetOptions, PanicStrategy};

pub fn target() -> Result<Target, String> {
    let opts = TargetOptions {
        // we allow dynamic linking, but only cdylibs. Basically we allow a
        // final library artifact that exports some symbols (a wasm module) but
        // we don't allow intermediate `dylib` crate types
        dynamic_linking: true,
        only_cdylib: true,

        // This means we'll just embed a `start` function in the wasm module
        executables: true,

        // relatively self-explanatory!
        exe_suffix: ".wasm".to_string(),
        dll_prefix: String::new(),
        dll_suffix: ".wasm".to_string(),
        linker_is_gnu: false,

        max_atomic_width: Some(64),

        // Unwinding doesn't work right now, so the whole target unconditionally
        // defaults to panic=abort. Note that this is guaranteed to change in
        // the future once unwinding is implemented. Don't rely on this.
        panic_strategy: PanicStrategy::Abort,

        // Wasm doesn't have atomics yet, so tell LLVM that we're in a single
        // threaded model which will legalize atomics to normal operations.
        singlethread: true,

        // no dynamic linking, no need for default visibility!
        default_hidden_visibility: true,

        // we use the LLD shipped with the Rust toolchain by default
        linker: Some("rust-lld".to_owned()),
        lld_flavor: LldFlavor::Wasm,

        // No need for indirection here, simd types can always be passed by
        // value as the whole module either has simd or not, which is different
        // from x86 (for example) where programs can have functions that don't
        // enable simd features.
        simd_types_indirect: false,

        .. Default::default()
    };
    Ok(Target {
        llvm_target: "wasm32-unknown-unknown".to_string(),
        target_endian: "little".to_string(),
        target_pointer_width: "32".to_string(),
        target_c_int_width: "32".to_string(),
        // This is basically guaranteed to change in the future, don't rely on
        // this. Use `not(target_os = "emscripten")` for now.
        target_os: "unknown".to_string(),
        target_env: String::new(),
        target_vendor: "unknown".to_string(),
        data_layout: "e-m:e-p:32:32-i64:64-n32:64-S128".to_string(),
        arch: "wasm32".to_string(),
        linker_flavor: LinkerFlavor::Lld(LldFlavor::Wasm),
        options: opts,
    })
}
