// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

static x : [isize; 4] = [1,2,3,4];
static y : &'static [isize] = &[1,2,3,4];
static z : &'static [isize; 4] = &[1,2,3,4];
static zz : &'static [isize] = &[1,2,3,4];

pub fn main() {
    println!("{}", x[1]);
    println!("{}", y[1]);
    println!("{}", z[1]);
    println!("{}", zz[1]);
    assert_eq!(x[1], 2);
    assert_eq!(x[3], 4);
    assert_eq!(x[3], y[3]);
    assert_eq!(z[1], 2);
    assert_eq!(z[3], 4);
    assert_eq!(z[3], y[3]);
    assert_eq!(zz[1], 2);
    assert_eq!(zz[3], 4);
    assert_eq!(zz[3], y[3]);
}
