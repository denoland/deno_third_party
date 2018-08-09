// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z parse-only

// Constants (static variables) can be used to match in patterns, but mutable
// statics cannot. This ensures that there's some form of error if this is
// attempted.

extern crate libc;

extern {
    static mut rust_dbg_static_mut: libc::c_int;
    pub fn rust_dbg_static_mut_check_four();
    #[cfg(stage37)] //~ ERROR expected item after attributes
}

pub fn main() {}
