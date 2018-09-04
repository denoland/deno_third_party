// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::sync::mpsc::channel;

pub fn main() { test00(); }

fn test00() {
    let _r: isize = 0;
    let mut sum: isize = 0;
    let (tx, rx) = channel();
    let number_of_messages: isize = 1000;
    let mut i: isize = 0;
    while i < number_of_messages { tx.send(i + 0).unwrap(); i += 1; }
    i = 0;
    while i < number_of_messages { sum += rx.recv().unwrap(); i += 1; }
    assert_eq!(sum, number_of_messages * (number_of_messages - 1) / 2);
}
