// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:extern-take-value.rs

extern crate extern_take_value;

pub fn main() {
    let a: extern "C" fn() -> i32 = extern_take_value::get_f();
    let b: extern "C" fn() -> i32 = extern_take_value::get_f();
    let c: extern "C" fn() -> i32 = extern_take_value::get_g();

    assert!(a == b);
    assert!(a != c);
}
