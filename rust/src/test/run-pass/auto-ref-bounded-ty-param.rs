// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait Foo {
    fn f(&self);
}

struct Bar {
    x: isize
}

trait Baz {
    fn g(&self);
}

impl<T:Baz> Foo for T {
    fn f(&self) {
        self.g();
    }
}

impl Baz for Bar {
    fn g(&self) {
        println!("{}", self.x);
    }
}

pub fn main() {
    let y = Bar { x: 42 };
    y.f();
}
