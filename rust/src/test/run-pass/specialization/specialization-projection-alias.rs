// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(specialization)]

// Regression test for ICE when combining specialized associated types and type
// aliases

trait Id_ {
    type Out;
}

type Id<T> = <T as Id_>::Out;

impl<T> Id_ for T {
    default type Out = T;
}

fn test_proection() {
    let x: Id<bool> = panic!();
}

fn main() {

}
