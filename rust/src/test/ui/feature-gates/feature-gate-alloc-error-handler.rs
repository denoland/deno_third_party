// compile-flags:-C panic=abort

#![no_std]
#![no_main]

use core::alloc::Layout;

#[alloc_error_handler] //~ ERROR #[alloc_error_handler] is an unstable feature (see issue #51540)
fn oom(info: Layout) -> ! {
    loop {}
}
