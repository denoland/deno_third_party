// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Makes sure that zero-initializing large types is reasonably fast,
// Doing it incorrectly causes massive slowdown in LLVM during
// optimisation.

// pretty-expanded FIXME #23616
// ignore-emscripten no threads support

#![feature(intrinsics, std_misc)]

use std::thread;

extern "rust-intrinsic" {
    pub fn init<T>() -> T;
}

const SIZE: usize = 1024 * 1024;

fn main() {
    // do the test in a new thread to avoid (spurious?) stack overflows
    thread::spawn(|| {
        let _memory: [u8; SIZE] = unsafe { init() };
    }).join();
}
