// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-emscripten no threads support

use std::thread;

static mut dropped: bool = false;

struct A {
    b: B,
}

struct B {
    foo: isize,
}

impl Drop for A {
    fn drop(&mut self) {
        panic!()
    }
}

impl Drop for B {
    fn drop(&mut self) {
        unsafe { dropped = true; }
    }
}

pub fn main() {
    let ret = thread::spawn(move|| {
        let _a = A { b: B { foo: 3 } };
    }).join();
    assert!(ret.is_err());
    unsafe { assert!(dropped); }
}
