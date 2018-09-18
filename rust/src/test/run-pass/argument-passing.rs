// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



struct X {
    x: isize
}

fn f1(a: &mut X, b: &mut isize, c: isize) -> isize {
    let r = a.x + *b + c;
    a.x = 0;
    *b = 10;
    return r;
}

fn f2<F>(a: isize, f: F) -> isize where F: FnOnce(isize) { f(1); return a; }

pub fn main() {
    let mut a = X {x: 1};
    let mut b = 2;
    let c = 3;
    assert_eq!(f1(&mut a, &mut b, c), 6);
    assert_eq!(a.x, 0);
    assert_eq!(b, 10);
    assert_eq!(f2(a.x, |_| a.x = 50), 0);
    assert_eq!(a.x, 50);
}
