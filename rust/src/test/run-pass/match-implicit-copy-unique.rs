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

struct Pair { a: Box<isize>, b: Box<isize> }

pub fn main() {
    let mut x: Box<_> = box Pair {a: box 10, b: box 20};
    let x_internal = &mut *x;
    match *x_internal {
      Pair {a: ref mut a, b: ref mut _b} => {
        assert_eq!(**a, 10);
        *a = box 30;
        assert_eq!(**a, 30);
      }
    }
}
