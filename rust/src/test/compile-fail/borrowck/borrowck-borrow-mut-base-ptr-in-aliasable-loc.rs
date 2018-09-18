// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that attempt to reborrow an `&mut` pointer in an aliasable
// location yields an error.
//
// Example from src/librustc_borrowck/borrowck/README.md

fn foo(t0: & &mut isize) {
    let t1 = t0;
    let p: &isize = &**t0;
    **t1 = 22; //~ ERROR cannot assign
}

fn foo3(t0: &mut &mut isize) {
    let t1 = &mut *t0;
    let p: &isize = &**t0; //~ ERROR cannot borrow
    **t1 = 22;
}

fn foo4(t0: & &mut isize) {
    let x:  &mut isize = &mut **t0; //~ ERROR cannot borrow
    *x += 1;
}

fn main() {
}
