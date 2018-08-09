// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:reexported_static_methods.rs

extern crate reexported_static_methods;

use reexported_static_methods::Foo;
use reexported_static_methods::Baz;
use reexported_static_methods::Boz;
use reexported_static_methods::Bort;

pub fn main() {
    assert_eq!(42_isize, Foo::foo());
    assert_eq!(84_isize, Baz::bar());
    assert!(Boz::boz(1));
    assert_eq!("bort()".to_string(), Bort::bort());
}
