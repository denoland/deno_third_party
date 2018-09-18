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

fn start(tx: &Sender<Sender<String>>) {
    let (tx2, rx) = channel();
    tx.send(tx2).unwrap();

    let mut a;
    let mut b;
    a = rx.recv().unwrap();
    assert_eq!(a, "A".to_string());
    println!("{}", a);
    b = rx.recv().unwrap();
    assert_eq!(b, "B".to_string());
    println!("{}", b);
}

pub fn main() {
    let (tx, rx) = channel();
    let child = thread::spawn(move|| { start(&tx) });

    let mut c = rx.recv().unwrap();
    c.send("A".to_string()).unwrap();
    c.send("B".to_string()).unwrap();
    thread::yield_now();

    child.join();
}
