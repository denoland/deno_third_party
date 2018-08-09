// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// exec-env:RUST_POISON_ON_FREE=1

// Test argument patterns where we create refs to the inside of
// boxes. Make sure that we don't free the box as we match the
// pattern.


#![allow(unknown_features)]
#![feature(box_patterns)]
#![feature(box_syntax)]

fn getaddr(box ref x: Box<usize>) -> *const usize {
    let addr: *const usize = &*x;
    addr
}

fn checkval(box ref x: Box<usize>) -> usize {
    *x
}

pub fn main() {
    let obj: Box<_> = box 1;
    let objptr: *const usize = &*obj;
    let xptr = getaddr(obj);
    assert_eq!(objptr, xptr);

    let obj = box 22;
    assert_eq!(checkval(obj), 22);
}
