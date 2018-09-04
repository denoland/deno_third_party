// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(slice_patterns)]

fn main() {
    assert_eq!(count_members(&[1, 2, 3, 4]), 4);
}

fn count_members(v: &[usize]) -> usize {
    match *v {
        []         => 0,
        [_]        => 1,
        [_, ref xs..] => 1 + count_members(xs)
    }
}
