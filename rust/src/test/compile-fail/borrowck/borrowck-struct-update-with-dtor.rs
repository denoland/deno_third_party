// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// revisions: ast mir
//[mir]compile-flags: -Z borrowck=mir

// Issue 4691: Ensure that functional-struct-update can only copy, not
// move, when the struct implements Drop.

struct B;
struct S { a: isize, b: B }
impl Drop for S { fn drop(&mut self) { } }

struct T { a: isize, mv: Box<isize> }
impl Drop for T { fn drop(&mut self) { } }

fn f(s0:S) {
    let _s2 = S{a: 2, ..s0};
    //[ast]~^ error: cannot move out of type `S`, which implements the `Drop` trait
    //[mir]~^^ ERROR [E0509]
}

fn g(s0:T) {
    let _s2 = T{a: 2, ..s0};
    //[ast]~^ error: cannot move out of type `T`, which implements the `Drop` trait
    //[mir]~^^ ERROR [E0509]
}

fn main() { }
