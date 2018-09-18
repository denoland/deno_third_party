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

// This file illustrates two things:
//
// 1. Only types that appear in a monomorphized function appear in the
//    print-type-sizes output, and
//
// 2. For an enum, the print-type-sizes output will also include the
//    size of each variant.

#![feature(start)]

pub struct SevenBytes([u8;  7]);
pub struct FiftyBytes([u8; 50]);

pub enum Enum {
    Small(SevenBytes),
    Large(FiftyBytes),
}

#[start]
fn start(_: isize, _: *const *const u8) -> isize {
    let _e: Enum;
    0
}
