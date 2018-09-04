// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Testing that this impl turns A into a Quux, because
// A is already a Foo Bar Baz

impl<T:Foo + Bar + Baz> Quux for T { }

trait Foo { fn f(&self) -> isize; }
trait Bar { fn g(&self) -> isize; }
trait Baz { fn h(&self) -> isize; }

trait Quux: Foo + Bar + Baz { }

struct A { x: isize }

impl Foo for A { fn f(&self) -> isize { 10 } }
impl Bar for A { fn g(&self) -> isize { 20 } }
impl Baz for A { fn h(&self) -> isize { 30 } }

fn f<T:Quux>(a: &T) {
    assert_eq!(a.f(), 10);
    assert_eq!(a.g(), 20);
    assert_eq!(a.h(), 30);
}

pub fn main() {
    let a = &A { x: 3 };
    f(a);
}
