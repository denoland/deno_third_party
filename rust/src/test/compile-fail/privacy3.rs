// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(start, no_core)]
#![no_core] // makes debugging this test *a lot* easier (during resolve)

// Test to make sure that private items imported through globs remain private
// when  they're used.

mod bar {
    pub use self::glob::*;

    mod glob {
        fn gpriv() {}
    }
}

pub fn foo() {}

fn test1() {
    use bar::gpriv;
    //~^ ERROR unresolved import `bar::gpriv` [E0432]
    //~| no `gpriv` in `bar`

    // This should pass because the compiler will insert a fake name binding
    // for `gpriv`
    gpriv();
}

#[start] fn main(_: isize, _: *const *const u8) -> isize { 3 }
