// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(core_intrinsics)]
#![allow(warnings)]

use std::intrinsics;

#[derive(Copy, Clone)]
struct Wrap(i64);

// These volatile intrinsics used to cause an ICE

unsafe fn test_bool(p: &mut bool, v: bool) {
    intrinsics::volatile_load(p);
    intrinsics::volatile_store(p, v);
}

unsafe fn test_immediate_fca(p: &mut Wrap, v: Wrap) {
    intrinsics::volatile_load(p);
    intrinsics::volatile_store(p, v);
}

fn main() {}
