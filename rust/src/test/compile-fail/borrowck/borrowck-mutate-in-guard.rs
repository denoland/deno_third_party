// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

enum Enum<'a> {
    A(&'a isize),
    B(bool),
}

fn foo() -> isize {
    let mut n = 42;
    let mut x = Enum::A(&mut n);
    match x {
        Enum::A(_) if { x = Enum::B(false); false } => 1,
        //~^ ERROR cannot assign in a pattern guard
        Enum::A(_) if { let y = &mut x; *y = Enum::B(false); false } => 1,
        //~^ ERROR cannot mutably borrow in a pattern guard
        //~^^ ERROR cannot assign in a pattern guard
        Enum::A(p) => *p,
        Enum::B(_) => 2,
    }
}

fn main() {
    foo();
}
