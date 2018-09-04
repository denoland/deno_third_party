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

use std::thread;

pub fn main() {
    let t = thread::spawn(move|| child((10, 20, 30, 40, 50, 60, 70, 80, 90)) );
    t.join().ok().unwrap(); // forget Err value, since it doesn't implement Debug
}

fn child(args: (isize, isize, isize, isize, isize, isize, isize, isize, isize)) {
    let (i1, i2, i3, i4, i5, i6, i7, i8, i9) = args;
    println!("{}", i1);
    println!("{}", i2);
    println!("{}", i3);
    println!("{}", i4);
    println!("{}", i5);
    println!("{}", i6);
    println!("{}", i7);
    println!("{}", i8);
    println!("{}", i9);
    assert_eq!(i1, 10);
    assert_eq!(i2, 20);
    assert_eq!(i3, 30);
    assert_eq!(i4, 40);
    assert_eq!(i5, 50);
    assert_eq!(i6, 60);
    assert_eq!(i7, 70);
    assert_eq!(i8, 80);
    assert_eq!(i9, 90);
}
