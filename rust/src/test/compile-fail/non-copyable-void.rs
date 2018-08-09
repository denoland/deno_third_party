// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
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

fn main() {
    let x : *const Vec<isize> = &vec![1,2,3];
    let y : *const libc::c_void = x as *const libc::c_void;
    unsafe {
        let _z = (*y).clone();
        //~^ ERROR no method named `clone` found
    }
}
