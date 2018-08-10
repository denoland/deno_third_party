// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

enum E {
    Foo,
    Bar(String)
}

struct S {
    x: E
}

fn f(x: String) {}

fn main() {
    let s = S { x: E::Bar("hello".to_string()) };
    match &s.x {
        &E::Foo => {}
        &E::Bar(identifier) => f(identifier.clone())  //~ ERROR cannot move
    };
    match &s.x {
        &E::Foo => {}
        &E::Bar(ref identifier) => println!("{}", *identifier)
    };
}
