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

trait get {
    fn get(self) -> isize;
}

// Note: impl on a slice; we're checking that the pointers below
// correctly get borrowed to `&`. (similar to impling for `isize`, with
// `&self` instead of `self`.)
impl<'a> get for &'a isize {
    fn get(self) -> isize {
        return *self;
    }
}

pub fn main() {
    let x: Box<_> = box 6;
    let y = x.get();
    println!("y={}", y);
    assert_eq!(y, 6);

    let x = &6;
    let y = x.get();
    println!("y={}", y);
    assert_eq!(y, 6);
}
