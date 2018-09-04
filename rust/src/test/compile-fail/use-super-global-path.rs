// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unused)]

struct S;
struct Z;

mod foo {
    use ::super::{S, Z}; //~ ERROR global paths cannot start with `super`

    pub fn g() {
        use ::super::main; //~ ERROR global paths cannot start with `super`
        main(); //~ ERROR cannot find function `main` in this scope
    }
}

fn main() { foo::g(); }
