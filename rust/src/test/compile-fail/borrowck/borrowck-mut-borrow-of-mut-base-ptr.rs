// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that attempt to mutably borrow `&mut` pointer while pointee is
// borrowed yields an error.
//
// Example from src/librustc_borrowck/borrowck/README.md

fn foo<'a>(mut t0: &'a mut isize,
           mut t1: &'a mut isize) {
    let p: &isize = &*t0;     // Freezes `*t0`
    let mut t2 = &mut t0;   //~ ERROR cannot borrow `t0`
    **t2 += 1;              // Mutates `*t0`
}

fn bar<'a>(mut t0: &'a mut isize,
           mut t1: &'a mut isize) {
    let p: &mut isize = &mut *t0; // Claims `*t0`
    let mut t2 = &mut t0;       //~ ERROR cannot borrow `t0`
    **t2 += 1;                  // Mutates `*t0` but not through `*p`
}

fn main() {
}
