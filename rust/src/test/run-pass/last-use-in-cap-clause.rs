// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Make sure #1399 stays fixed

struct A { a: Box<isize> }

fn foo() -> Box<FnMut() -> isize + 'static> {
    let k: Box<_> = Box::new(22);
    let _u = A {a: k.clone()};
    let result  = || 22;
    Box::new(result)
}

pub fn main() {
    assert_eq!(foo()(), 22);
}
