// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Make sure #1399 stays fixed

#![allow(unknown_features)]
#![feature(box_syntax)]

struct A { a: Box<isize> }

pub fn main() {
    fn invoke<F>(f: F) where F: FnOnce() { f(); }
    let k: Box<_> = box 22;
    let _u = A {a: k.clone()};
    invoke(|| println!("{}", k.clone()) )
}
