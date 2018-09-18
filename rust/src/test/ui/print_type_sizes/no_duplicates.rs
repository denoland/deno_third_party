// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z print-type-sizes
// compile-pass

// This file illustrates that when the same type occurs repeatedly
// (even if multiple functions), it is only printed once in the
// print-type-sizes output.

#![feature(start)]

pub struct SevenBytes([u8; 7]);

pub fn f1() {
    let _s: SevenBytes = SevenBytes([0; 7]);
}

#[start]
fn start(_: isize, _: *const *const u8) -> isize {
    let _s: SevenBytes = SevenBytes([0; 7]);
    0
}
