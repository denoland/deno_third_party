// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct T { i: i32 }
fn f<T>() {
    let t = T { i: 0 }; //~ ERROR expected struct, variant or union type, found type parameter `T`
}

mod Foo {
    pub fn f() {}
}
fn g<Foo>() {
    Foo::f(); //~ ERROR no function or associated item named `f`
}

fn main() {}
