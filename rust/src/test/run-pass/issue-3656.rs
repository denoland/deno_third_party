// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Issue #3656
// Incorrect struct size computation in the FFI, because of not taking
// the alignment of elements into account.

// pretty-expanded FIXME #23616
// ignore-wasm32-bare no libc to test with

#![feature(libc)]

extern crate libc;
use libc::{c_uint, uint32_t, c_void};

pub struct KEYGEN {
    hash_algorithm: [c_uint; 2],
    count: uint32_t,
    salt: *const c_void,
    salt_size: uint32_t,
}

extern {
    // Bogus signature, just need to test if it compiles.
    pub fn malloc(data: KEYGEN);
}

pub fn main() {
}
