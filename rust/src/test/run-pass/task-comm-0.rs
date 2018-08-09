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

pub fn main() { test05(); }

fn test05_start(tx : &Sender<isize>) {
    tx.send(10).unwrap();
    println!("sent 10");
    tx.send(20).unwrap();
    println!("sent 20");
    tx.send(30).unwrap();
    println!("sent 30");
}

fn test05() {
    let (tx, rx) = channel();
    let t = thread::spawn(move|| { test05_start(&tx) });
    let mut value: isize = rx.recv().unwrap();
    println!("{}", value);
    value = rx.recv().unwrap();
    println!("{}", value);
    value = rx.recv().unwrap();
    println!("{}", value);
    assert_eq!(value, 30);
    t.join();
}
