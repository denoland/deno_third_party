// pretty-expanded FIXME #23616

// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unnecessary_allocation)]
#![allow(unknown_features)]
#![feature(box_syntax)]

// Tests for a previous bug that occurred due to an interaction
// between struct field initialization and the auto-coercion
// from a vector to a slice. The drop glue was being invoked on
// the temporary slice with a wrong type, triggering an LLVM assert.


struct Thing1<'a> {
    baz: &'a [Box<isize>],
    bar: Box<u64>,
}

struct Thing2<'a> {
    baz: &'a [Box<isize>],
    bar: u64,
}

pub fn main() {
    let _t1_fixed = Thing1 {
        baz: &[],
        bar: box 32,
    };
    Thing1 {
        baz: &Vec::new(),
        bar: box 32,
    };
    let _t2_fixed = Thing2 {
        baz: &[],
        bar: 32,
    };
    Thing2 {
        baz: &Vec::new(),
        bar: 32,
    };
}
