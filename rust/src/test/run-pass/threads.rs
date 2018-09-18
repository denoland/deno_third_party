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

#![feature(std_misc)]

use std::thread;

pub fn main() {
    let mut i = 10;
    while i > 0 {
        thread::spawn({let i = i; move|| child(i)}).join();
        i = i - 1;
    }
    println!("main thread exiting");
}

fn child(x: isize) { println!("{}", x); }
