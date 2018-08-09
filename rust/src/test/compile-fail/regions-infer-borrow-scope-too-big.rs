// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


struct point {
    x: isize,
    y: isize,
}

fn x_coord<'r>(p: &'r point) -> &'r isize {
    return &p.x;
}

fn foo<'a>(p: Box<point>) -> &'a isize {
    let xc = x_coord(&*p); //~ ERROR `*p` does not live long enough
    assert_eq!(*xc, 3);
    return xc;
}

fn main() {}
