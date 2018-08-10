// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:cci_iter_lib.rs

extern crate cci_iter_lib;

pub fn main() {
    //let bt0 = sys::rusti::frame_address(1);
    //println!("%?", bt0);
    cci_iter_lib::iter(&[1, 2, 3], |i| {
        println!("{}", *i);
        //assert_eq!(bt0, sys::rusti::frame_address(2));
    })
}
