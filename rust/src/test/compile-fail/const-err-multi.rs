// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(const_err)]

pub const A: i8 = -std::i8::MIN;
//~^ ERROR E0080
//~| ERROR attempt to negate with overflow
//~| ERROR this expression will panic at runtime
//~| ERROR this constant cannot be used
pub const B: i8 = A;
//~^ ERROR const_err
//~| ERROR const_err
pub const C: u8 = A as u8;
//~^ ERROR const_err
//~| ERROR const_err
pub const D: i8 = 50 - A;
//~^ ERROR const_err
//~| ERROR const_err

fn main() {
    let _ = (A, B, C, D);
}
