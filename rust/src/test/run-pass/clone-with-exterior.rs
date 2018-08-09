// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-emscripten no threads support

#![allow(unknown_features)]
#![feature(box_syntax, std_misc)]

use std::thread;

struct Pair {
    a: isize,
    b: isize
}

pub fn main() {
    let z: Box<_> = box Pair { a : 10, b : 12};

    thread::spawn(move|| {
        assert_eq!(z.a, 10);
        assert_eq!(z.b, 12);
    }).join();
}
