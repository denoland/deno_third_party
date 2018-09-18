// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#![allow(unknown_features)]
#![feature(box_syntax)]

use std::default::Default;

#[derive(Default)]
struct A {
    foo: Box<[bool]>,
}

pub fn main() {
    let a: A = Default::default();
    let b: Box<[_]> = Box::<[bool; 0]>::new([]);
    assert_eq!(a.foo, b);
}
