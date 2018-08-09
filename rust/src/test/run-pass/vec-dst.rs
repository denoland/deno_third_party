// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
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

pub fn main() {
    // Tests for indexing into box/& [T; n]
    let x: [isize; 3] = [1, 2, 3];
    let mut x: Box<[isize; 3]> = box x;
    assert_eq!(x[0], 1);
    assert_eq!(x[1], 2);
    assert_eq!(x[2], 3);
    x[1] = 45;
    assert_eq!(x[0], 1);
    assert_eq!(x[1], 45);
    assert_eq!(x[2], 3);

    let mut x: [isize; 3] = [1, 2, 3];
    let x: &mut [isize; 3] = &mut x;
    assert_eq!(x[0], 1);
    assert_eq!(x[1], 2);
    assert_eq!(x[2], 3);
    x[1] = 45;
    assert_eq!(x[0], 1);
    assert_eq!(x[1], 45);
    assert_eq!(x[2], 3);
}
