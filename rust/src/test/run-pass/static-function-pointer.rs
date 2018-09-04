// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


fn f(x: isize) -> isize { x }
fn g(x: isize) -> isize { 2 * x }

static F: fn(isize) -> isize = f;
static mut G: fn(isize) -> isize = f;

pub fn main() {
    assert_eq!(F(42), 42);
    unsafe {
        assert_eq!(G(42), 42);
        G = g;
        assert_eq!(G(42), 84);
    }
}
