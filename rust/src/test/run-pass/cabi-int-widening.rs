// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-wasm32-bare no libc to test ffi with

#[link(name = "rust_test_helpers", kind = "static")]
extern {
    fn rust_int8_to_int32(_: i8) -> i32;
}

fn main() {
    let x = unsafe {
        rust_int8_to_int32(-1)
    };

    assert!(x == -1);
}
