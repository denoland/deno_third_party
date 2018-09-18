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
// pretty-expanded FIXME #23616

#![feature(libc)]

extern crate libc;

mod bar {
    extern {}
}

mod zed {
    extern {}
}

mod mlibc {
    use libc::{c_int, c_void, size_t, ssize_t};

    extern {
        pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
    }
}

mod baz {
    extern {}
}

pub fn main() { }
