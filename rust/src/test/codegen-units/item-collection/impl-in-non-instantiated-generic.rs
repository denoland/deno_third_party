// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-tidy-linelength
// compile-flags:-Zprint-mono-items=eager

#![deny(dead_code)]
#![feature(start)]

trait SomeTrait {
    fn foo(&self);
}

// This function is never instantiated but the contained impl must still be
// discovered.
pub fn generic_function<T>(x: T) -> (T, i32) {
    impl SomeTrait for i64 {
        //~ MONO_ITEM fn impl_in_non_instantiated_generic::generic_function[0]::{{impl}}[0]::foo[0]
        fn foo(&self) {}
    }

    (x, 0)
}

//~ MONO_ITEM fn impl_in_non_instantiated_generic::start[0]
#[start]
fn start(_: isize, _: *const *const u8) -> isize {
    0i64.foo();

    0
}
