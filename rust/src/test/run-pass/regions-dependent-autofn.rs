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

// pretty-expanded FIXME #23616

fn subslice<F>(v: F) -> F where F: FnOnce() { v }

fn both<F>(v: F) -> F where F: FnOnce() {
    subslice(subslice(v))
}

pub fn main() {
    both(main);
}
