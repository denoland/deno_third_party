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
#![feature(box_syntax)]

#[derive(Clone)]
struct Triple {
    x: isize,
    y: isize,
    z: isize,
}

fn test(x: bool, foo: Box<Triple>) -> isize {
    let bar = foo;
    let mut y: Box<Triple>;
    if x { y = bar; } else { y = box Triple {x: 4, y: 5, z: 6}; }
    return y.y;
}

pub fn main() {
    let x: Box<_> = box Triple{x: 1, y: 2, z: 3};
    for _ in 0_usize..10000_usize {
        assert_eq!(test(true, x.clone()), 2);
    }
    assert_eq!(test(false, x), 5);
}
