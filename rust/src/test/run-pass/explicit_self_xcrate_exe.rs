// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:explicit_self_xcrate.rs

// pretty-expanded FIXME #23616

extern crate explicit_self_xcrate;
use explicit_self_xcrate::{Foo, Bar};

pub fn main() {
    let x = Bar { x: "hello".to_string() };
    x.f();
}
