// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-emscripten no threads support

// Test that if a slicing expr[..] fails, the correct cleanups happen.


use std::thread;

struct Foo;

static mut DTOR_COUNT: isize = 0;

impl Drop for Foo {
    fn drop(&mut self) { unsafe { DTOR_COUNT += 1; } }
}

fn foo() {
    let x: &[_] = &[Foo, Foo];
    &x[3..4];
}

fn main() {
    let _ = thread::spawn(move|| foo()).join();
    unsafe { assert_eq!(DTOR_COUNT, 2); }
}
