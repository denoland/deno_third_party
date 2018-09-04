// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Regression test for issue #45045

#![feature(nll)]

enum Xyz {
    A,
    B,
}

fn main() {
    let mut e = Xyz::A;
    let f = &mut e;
    let g = f;
    match e { //~ cannot use `e` because it was mutably borrowed [E0503]
        Xyz::A => println!("a"),
        //~^ cannot use `e` because it was mutably borrowed [E0503]
        Xyz::B => println!("b"),
    };
    *g = Xyz::B;
}
