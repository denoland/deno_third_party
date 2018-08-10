// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:linkage-visibility.rs
// ignore-android: FIXME(#10356)
// ignore-windows: std::dynamic_lib does not work on Windows well
// ignore-emscripten no dynamic linking

extern crate linkage_visibility as foo;

pub fn main() {
    foo::test();
    foo::foo2::<isize>();
    foo::foo();
}
