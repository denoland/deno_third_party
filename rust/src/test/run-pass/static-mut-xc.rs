// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Constants (static variables) can be used to match in patterns, but mutable
// statics cannot. This ensures that there's some form of error if this is
// attempted.

// aux-build:static_mut_xc.rs


extern crate static_mut_xc;

unsafe fn static_bound(_: &'static isize) {}

fn static_bound_set(a: &'static mut isize) {
    *a = 3;
}

unsafe fn run() {
    assert_eq!(static_mut_xc::a, 3);
    static_mut_xc::a = 4;
    assert_eq!(static_mut_xc::a, 4);
    static_mut_xc::a += 1;
    assert_eq!(static_mut_xc::a, 5);
    static_mut_xc::a *= 3;
    assert_eq!(static_mut_xc::a, 15);
    static_mut_xc::a = -3;
    assert_eq!(static_mut_xc::a, -3);
    static_bound(&static_mut_xc::a);
    static_bound_set(&mut static_mut_xc::a);
}

pub fn main() {
    unsafe { run() }
}

pub mod inner {
    pub static mut a: isize = 4;
}
