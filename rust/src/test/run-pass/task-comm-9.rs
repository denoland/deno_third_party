// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
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
use std::sync::mpsc::{channel, Sender};

pub fn main() { test00(); }

fn test00_start(c: &Sender<isize>, number_of_messages: isize) {
    let mut i: isize = 0;
    while i < number_of_messages { c.send(i + 0).unwrap(); i += 1; }
}

fn test00() {
    let r: isize = 0;
    let mut sum: isize = 0;
    let (tx, rx) = channel();
    let number_of_messages: isize = 10;

    let result = thread::spawn(move|| {
        test00_start(&tx, number_of_messages);
    });

    let mut i: isize = 0;
    while i < number_of_messages {
        sum += rx.recv().unwrap();
        println!("{}", r);
        i += 1;
    }

    result.join();

    assert_eq!(sum, number_of_messages * (number_of_messages - 1) / 2);
}
