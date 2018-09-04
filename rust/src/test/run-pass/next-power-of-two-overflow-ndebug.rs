// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -C debug_assertions=no
// ignore-emscripten dies with an LLVM error

fn main() {
    for i in 129..256 {
        assert_eq!((i as u8).next_power_of_two(), 0);
    }

    assert_eq!(((1u16 << 15) + 1).next_power_of_two(), 0);
    assert_eq!(((1u32 << 31) + 1).next_power_of_two(), 0);
    assert_eq!(((1u64 << 63) + 1).next_power_of_two(), 0);
    assert_eq!(((1u128 << 127) + 1).next_power_of_two(), 0);
}
