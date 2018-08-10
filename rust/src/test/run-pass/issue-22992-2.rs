// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct A(B);
struct B;

use std::ops::Deref;

impl Deref for A {
    type Target = B;
    fn deref(&self) -> &B { &self.0 }
}

impl B {
    fn foo(&self) {}
}

fn main() {
    A(B).foo();
}
