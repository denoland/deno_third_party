// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



fn f(arg: &mut A) {
    arg.a = 100;
}

#[derive(Copy, Clone)]
struct A { a: isize }

pub fn main() {
    let mut x = A {a: 10};
    f(&mut x);
    assert_eq!(x.a, 100);
    x.a = 20;
    let mut y = x;
    f(&mut y);
    assert_eq!(x.a, 20);
}
