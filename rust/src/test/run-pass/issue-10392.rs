// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


struct A { foo: isize }
struct B { a: isize, b: isize, c: isize }

fn mka() -> A { panic!() }
fn mkb() -> B { panic!() }

fn test() {
    let A { foo, } = mka();
    let A {
        foo,
    } = mka();

    let B { a, b, c, } = mkb();

    match mka() {
        A { foo: _foo, } => {}
    }

    match Some(mka()) {
        Some(A { foo: _foo, }) => {}
        None => {}
    }
}

pub fn main() {
    if false { test() }
}
