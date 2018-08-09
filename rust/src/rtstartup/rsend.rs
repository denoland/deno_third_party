// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// See rsbegin.rs for details.

#![feature(no_core, lang_items, optin_builtin_traits)]
#![crate_type="rlib"]
#![no_core]

#[lang = "sized"]
trait Sized {}
#[lang = "sync"]
trait Sync {}
impl<T> Sync for T {}
#[lang = "copy"]
trait Copy {}
#[lang = "freeze"]
auto trait Freeze {}

#[lang = "drop_in_place"]
#[inline]
#[allow(unconditional_recursion)]
pub unsafe fn drop_in_place<T: ?Sized>(to_drop: *mut T) {
    drop_in_place(to_drop);
}

#[cfg(all(target_os="windows", target_arch = "x86", target_env="gnu"))]
pub mod eh_frames {
    // Terminate the frame unwind info section with a 0 as a sentinel;
    // this would be the 'length' field in a real FDE.
    #[no_mangle]
    #[link_section = ".eh_frame"]
    pub static __EH_FRAME_END__: u32 = 0;
}
