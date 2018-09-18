// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(untagged_unions)]

union U1 {
    a: u8
}

union U2 {
    a: String
}

union U3<T> {
    a: T
}

union U4<T: Copy> {
    a: T
}

fn generic_noncopy<T: Default>() {
    let mut u3 = U3 { a: T::default() };
    u3.a = T::default(); //~ ERROR assignment to non-`Copy` union field requires unsafe
}

fn generic_copy<T: Copy + Default>() {
    let mut u3 = U3 { a: T::default() };
    u3.a = T::default(); // OK
    let mut u4 = U4 { a: T::default() };
    u4.a = T::default(); // OK
}

fn main() {
    let mut u1 = U1 { a: 10 }; // OK
    let a = u1.a; //~ ERROR access to union field requires unsafe
    u1.a = 11; // OK
    let U1 { a } = u1; //~ ERROR access to union field requires unsafe
    if let U1 { a: 12 } = u1 {} //~ ERROR access to union field requires unsafe
    // let U1 { .. } = u1; // OK

    let mut u2 = U2 { a: String::from("old") }; // OK
    u2.a = String::from("new"); //~ ERROR assignment to non-`Copy` union field requires unsafe
    let mut u3 = U3 { a: 0 }; // OK
    u3.a = 1; // OK
    let mut u3 = U3 { a: String::from("old") }; // OK
    u3.a = String::from("new"); //~ ERROR assignment to non-`Copy` union field requires unsafe
}
