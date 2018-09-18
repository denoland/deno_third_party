// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

const XY_1: i32 = 10;

fn main() {
    const XY_2: i32 = 20;
    let a = concat_idents!(X, Y_1); //~ ERROR `concat_idents` is not stable
    let b = concat_idents!(X, Y_2); //~ ERROR `concat_idents` is not stable
    assert_eq!(a, 10);
    assert_eq!(b, 20);
}
