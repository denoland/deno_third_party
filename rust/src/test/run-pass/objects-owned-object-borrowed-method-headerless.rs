// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test invoked `&self` methods on owned objects where the values
// closed over do not contain managed values, and thus the boxes do
// not have headers.


#![allow(unknown_features)]
#![feature(box_syntax)]


trait FooTrait {
    fn foo(&self) -> usize;
}

struct BarStruct {
    x: usize
}

impl FooTrait for BarStruct {
    fn foo(&self) -> usize {
        self.x
    }
}

pub fn main() {
    let foos: Vec<Box<FooTrait>> = vec![
        box BarStruct{ x: 0 } as Box<FooTrait>,
        box BarStruct{ x: 1 } as Box<FooTrait>,
        box BarStruct{ x: 2 } as Box<FooTrait>
    ];

    for i in 0..foos.len() {
        assert_eq!(i, foos[i].foo());
    }
}
