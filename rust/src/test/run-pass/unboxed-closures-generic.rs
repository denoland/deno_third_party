// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::FnMut;

fn call_it<F:FnMut(i32,i32)->i32>(y: i32, mut f: F) -> i32 {
    f(2, y)
}

pub fn main() {
    let f = |x: i32, y: i32| -> i32 { x + y };
    let z = call_it(3, f);
    println!("{}", z);
    assert_eq!(z, 5);
}
