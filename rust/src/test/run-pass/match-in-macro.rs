// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


enum Foo {
    B { b1: isize, bb1: isize},
}

macro_rules! match_inside_expansion {
    () => (
        match (Foo::B { b1:29 , bb1: 100}) {
            Foo::B { b1:b2 , bb1:bb2 } => b2+bb2
        }
    )
}

pub fn main() {
    assert_eq!(match_inside_expansion!(),129);
}
