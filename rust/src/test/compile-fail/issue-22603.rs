// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(unboxed_closures, fn_traits, rustc_attrs)]

struct Foo;

impl<A> FnOnce<(A,)> for Foo {
    type Output = ();
    extern "rust-call" fn call_once(self, (_,): (A,)) {
    }
}
#[rustc_error]
fn main() { //~ ERROR compilation successful
    println!("{:?}", Foo("bar"));
}
