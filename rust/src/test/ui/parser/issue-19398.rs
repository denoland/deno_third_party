// compile-flags: -Z parse-only

trait T {
    extern "Rust" unsafe fn foo(); //~ ERROR expected `fn`, found `unsafe`
}

fn main() {}
