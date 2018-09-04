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

use std::sync::mpsc::{channel, Sender};
use std::thread;

fn start(tx: &Sender<isize>, start: isize, number_of_messages: isize) {
    let mut i: isize = 0;
    while i< number_of_messages { tx.send(start + i).unwrap(); i += 1; }
}

pub fn main() {
    println!("Check that we don't deadlock.");
    let (tx, rx) = channel();
    let _ = thread::spawn(move|| { start(&tx, 0, 10) }).join();
    println!("Joined task");
}
