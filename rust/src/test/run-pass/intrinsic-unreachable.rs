// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(core_intrinsics)]

use std::intrinsics;

// See also src/test/run-make/intrinsic-unreachable.

unsafe fn f(x: usize) -> usize {
    match x {
        17 => 23,
        _ => intrinsics::unreachable(),
    }
}

fn main() {
    assert_eq!(unsafe { f(17) }, 23);
}
