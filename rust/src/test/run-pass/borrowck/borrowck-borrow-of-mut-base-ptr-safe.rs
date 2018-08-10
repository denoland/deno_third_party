// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that freezing an `&mut` pointer while referent is
// frozen is legal.
//
// Example from src/librustc_borrowck/borrowck/README.md

// pretty-expanded FIXME #23616

fn foo<'a>(mut t0: &'a mut isize,
           mut t1: &'a mut isize) {
    let p: &isize = &*t0; // Freezes `*t0`
    let mut t2 = &t0;
    let q: &isize = &**t2; // Freezes `*t0`, but that's ok...
    let r: &isize = &*t0; // ...after all, could do same thing directly.
}

pub fn main() {
}
