// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(exceeding_bitshifts, const_err)]
#![allow(unused_variables)]
#![allow(dead_code)]

fn main() {
      let n = 1u8 << 7;
      let n = 1u8 << 8;   //~ ERROR: attempt to shift left with overflow
      let n = 1u16 << 15;
      let n = 1u16 << 16; //~ ERROR: attempt to shift left with overflow
      let n = 1u32 << 31;
      let n = 1u32 << 32; //~ ERROR: attempt to shift left with overflow
      let n = 1u64 << 63;
      let n = 1u64 << 64; //~ ERROR: attempt to shift left with overflow
      let n = 1i8 << 7;
      let n = 1i8 << 8;   //~ ERROR: attempt to shift left with overflow
      let n = 1i16 << 15;
      let n = 1i16 << 16; //~ ERROR: attempt to shift left with overflow
      let n = 1i32 << 31;
      let n = 1i32 << 32; //~ ERROR: attempt to shift left with overflow
      let n = 1i64 << 63;
      let n = 1i64 << 64; //~ ERROR: attempt to shift left with overflow

      let n = 1u8 >> 7;
      let n = 1u8 >> 8;   //~ ERROR: attempt to shift right with overflow
      let n = 1u16 >> 15;
      let n = 1u16 >> 16; //~ ERROR: attempt to shift right with overflow
      let n = 1u32 >> 31;
      let n = 1u32 >> 32; //~ ERROR: attempt to shift right with overflow
      let n = 1u64 >> 63;
      let n = 1u64 >> 64; //~ ERROR: attempt to shift right with overflow
      let n = 1i8 >> 7;
      let n = 1i8 >> 8;   //~ ERROR: attempt to shift right with overflow
      let n = 1i16 >> 15;
      let n = 1i16 >> 16; //~ ERROR: attempt to shift right with overflow
      let n = 1i32 >> 31;
      let n = 1i32 >> 32; //~ ERROR: attempt to shift right with overflow
      let n = 1i64 >> 63;
      let n = 1i64 >> 64; //~ ERROR: attempt to shift right with overflow

      let n = 1u8;
      let n = n << 7;
      let n = n << 8; //~ ERROR: attempt to shift left with overflow

      let n = 1u8 << -8; //~ ERROR: attempt to shift left with overflow

      let n = 1i8<<(1isize+-1);
}
