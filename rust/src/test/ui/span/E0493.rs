// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct Foo {
    a: u32
}

impl Drop for Foo {
    fn drop(&mut self) {}
}

struct Bar {
    a: u32
}

impl Drop for Bar {
    fn drop(&mut self) {}
}

const F : Foo = (Foo { a : 0 }, Foo { a : 1 }).1;
//~^ destructors cannot be evaluated at compile-time

fn main() {
}
