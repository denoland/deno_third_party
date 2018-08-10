// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-wasm32-bare no libc to test ffi with

#![feature(libc)]

extern crate libc;

use std::mem;
use libc::{c_double, c_int};

fn to_c_int(v: &mut isize) -> &mut c_int {
    unsafe {
        mem::transmute_copy(&v)
    }
}

fn lgamma(n: c_double, value: &mut isize) -> c_double {
    unsafe {
        return m::lgamma(n, to_c_int(value));
    }
}

mod m {
    use libc::{c_double, c_int};

    #[link_name = "m"]
    extern {
        #[cfg(any(unix, target_os = "cloudabi"))]
        #[link_name="lgamma_r"]
        pub fn lgamma(n: c_double, sign: &mut c_int) -> c_double;
        #[cfg(windows)]
        #[link_name="lgamma"]
        pub fn lgamma(n: c_double, sign: &mut c_int) -> c_double;
    }
}

pub fn main() {
  let mut y: isize = 5;
  let x: &mut isize = &mut y;
  assert_eq!(lgamma(1.0 as c_double, x), 0.0 as c_double);
}
