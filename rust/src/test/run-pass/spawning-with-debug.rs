// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-windows
// exec-env:RUST_LOG=debug
// ignore-emscripten no threads support

// regression test for issue #10405, make sure we don't call println! too soon.

use std::thread::Builder;

pub fn main() {
    let mut t = Builder::new();
    t.spawn(move|| ());
}
