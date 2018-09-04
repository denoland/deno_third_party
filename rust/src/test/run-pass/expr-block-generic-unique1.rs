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

fn test_generic<T, F>(expected: Box<T>, eq: F) where T: Clone, F: FnOnce(Box<T>, Box<T>) -> bool {
    let actual: Box<T> = { expected.clone() };
    assert!(eq(expected, actual));
}

fn test_box() {
    fn compare_box(b1: Box<bool>, b2: Box<bool>) -> bool {
        println!("{}", *b1);
        println!("{}", *b2);
        return *b1 == *b2;
    }
    test_generic::<bool, _>(box true, compare_box);
}

pub fn main() { test_box(); }
