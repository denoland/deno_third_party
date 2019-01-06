// aux-build:test-macros.rs
// ignore-wasm32

extern crate test_macros;

use test_macros::{nop_attr, no_output, emit_input};

fn main() {
    assert_eq!(unsafe { rust_get_test_int() }, 0isize);
    assert_eq!(unsafe { rust_dbg_extern_identity_u32(0xDEADBEEF) }, 0xDEADBEEF);
}

#[link(name = "rust_test_helpers", kind = "static")]
extern {
    #[no_output]
    //~^ ERROR macro invocations in `extern {}` blocks are experimental
    fn some_definitely_unknown_symbol_which_should_be_removed();

    #[nop_attr]
    //~^ ERROR macro invocations in `extern {}` blocks are experimental
    fn rust_get_test_int() -> isize;

    emit_input!(fn rust_dbg_extern_identity_u32(arg: u32) -> u32;);
    //~^ ERROR macro invocations in `extern {}` blocks are experimental
}
