// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// for this issue, this code must be built in a library

use std::mem;

trait A {
    fn dummy(&self) { }
}
struct B;
impl A for B {}

fn bar<T>(_: &mut A, _: &T) {}

fn foo<T>(t: &T) {
    let mut b = B;
    bar(&mut b as &mut A, t)
}
