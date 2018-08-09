// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Catch mistakes in the overflowing literals lint.
#![deny(overflowing_literals)]

pub fn main() {
    assert_eq!(0xffffffff, (!0 as u32));
    assert_eq!(4294967295, (!0 as u32));
    assert_eq!(0xffffffffffffffff, (!0 as u64));
    assert_eq!(18446744073709551615, (!0 as u64));

    assert_eq!((-2147483648i32).wrapping_sub(1), 2147483647);

    assert_eq!(-3.40282356e+38_f32, ::std::f32::MIN);
    assert_eq!(3.40282356e+38_f32, ::std::f32::MAX);
    assert_eq!(-1.7976931348623158e+308_f64, ::std::f64::MIN);
    assert_eq!(1.7976931348623158e+308_f64, ::std::f64::MAX);
}
