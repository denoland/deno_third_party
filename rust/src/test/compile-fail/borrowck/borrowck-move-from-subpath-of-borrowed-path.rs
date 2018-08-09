// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// verify that an error is raised when trying to move out of a
// borrowed path.

#![feature(box_syntax)]

fn main() {
    let a: Box<Box<_>> = box box 2;
    let b = &a;

    let z = *a; //~ ERROR: cannot move out of `*a` because it is borrowed
}
