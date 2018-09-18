// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test lifetimes are linked properly when we autoslice a vector.
// Issue #3148.

fn subslice1<'r>(v: &'r [usize]) -> &'r [usize] { v }

fn both<'r>(v: &'r [usize]) -> &'r [usize] {
    subslice1(subslice1(v))
}

pub fn main() {
    let v = vec![1,2,3];
    both(&v);
}
