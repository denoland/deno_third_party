// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we detect nested calls that could free pointers evaluated
// for earlier arguments.

#![feature(box_syntax)]

fn rewrite(v: &mut Box<usize>) -> usize {
    *v = box 22;
    **v
}

fn add(v: &usize, w: usize) -> usize {
    *v + w
}

fn implicit() {
    let mut a: Box<_> = box 1;

    // Note the danger here:
    //
    //    the pointer for the first argument has already been
    //    evaluated, but it gets freed when evaluating the second
    //    argument!
    add(
        &*a,
        rewrite(&mut a)); //~ ERROR cannot borrow
}

fn explicit() {
    let mut a: Box<_> = box 1;
    add(
        &*a,
        rewrite(&mut a)); //~ ERROR cannot borrow
}

fn main() {}
