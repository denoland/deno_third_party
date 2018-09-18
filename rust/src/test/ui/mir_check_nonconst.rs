// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

struct Foo { a: u8 }
fn bar() -> Foo {
    Foo { a: 5 }
}

static foo: Foo = bar();
//~^ ERROR calls in statics are limited to constant functions, tuple structs and tuple variants

fn main() {}
