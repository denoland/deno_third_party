// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-tidy-linelength
// compile-flags:-Zprint-mono-items=lazy

// NB: We do not expect *any* monomorphization to be generated here.

#![feature(const_fn)]
#![deny(dead_code)]
#![crate_type = "rlib"]

pub const fn foo(x: u32) -> u32 {
    x + 0xf00
}
