// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
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

// `move_val_init` has an odd desugaring, check that it is still treated
// as unsafe.
fn main() {
    intrinsics::move_val_init(1 as *mut u32, 1);
    //~^ ERROR dereference of raw pointer requires unsafe function or block
}
