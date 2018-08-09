// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that `fn foo::bar::{self}` only imports `bar` in the type namespace.

mod foo {
    pub fn f() { }
}
use foo::f::{self}; //~ ERROR unresolved import `foo::f`

mod bar {
    pub fn baz() {}
    pub mod baz {}
}
use bar::baz::{self};

fn main() {
    baz(); //~ ERROR expected function, found module `baz`
}
