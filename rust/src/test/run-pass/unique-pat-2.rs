// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#![allow(unknown_features)]
#![feature(box_patterns)]
#![feature(box_syntax)]

struct Foo {a: isize, b: usize}

enum bar { u(Box<Foo>), w(isize), }

pub fn main() {
    assert!(match bar::u(box Foo{a: 10, b: 40}) {
              bar::u(box Foo{a: a, b: b}) => { a + (b as isize) }
              _ => { 66 }
            } == 50);
}
