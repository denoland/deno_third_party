// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Tests that bindings to move-by-default values trigger moves of the
// discriminant. Also tests that the compiler explains the move in
// terms of the binding, not the discriminant.

struct Foo<A> { f: A }
fn guard(_s: String) -> bool {panic!()}
fn touch<A>(_a: &A) {}

fn f10() {
    let x = Foo {f: "hi".to_string()};

    let y = match x {
        Foo {f} => {}
    };

    touch(&x); //~ ERROR use of partially moved value: `x`
    //~^ value used here after move
    //~| move occurs because `x.f` has type `std::string::String`
}

fn main() {}
