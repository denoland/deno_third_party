// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

enum Foo {
    IntVal(i32),
    Int64Val(i64)
}

struct Bar {
    i: i32,
    v: Foo
}

static bar: Bar = Bar { i: 0, v: Foo::IntVal(0) };

pub fn main() {}
