// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(lang_items, start, no_core)]
#![no_core] // makes debugging this test *a lot* easier (during resolve)

#[lang = "sized"] pub trait Sized {}
#[lang="copy"] pub trait Copy {}

// Test to make sure that private items imported through globs remain private
// when  they're used.

mod bar {
    pub use self::glob::*;

    mod glob {
        fn gpriv() {}
    }
}

pub fn foo() {}

fn test2() {
    use bar::glob::gpriv; //~ ERROR: module `glob` is private
    gpriv();
}

#[start] fn main(_: isize, _: *const *const u8) -> isize { 3 }
