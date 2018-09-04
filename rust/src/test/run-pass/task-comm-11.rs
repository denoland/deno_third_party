// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616
// ignore-emscripten no threads support

#![feature(std_misc)]

use std::sync::mpsc::{channel, Sender};
use std::thread;

fn start(tx: &Sender<Sender<isize>>) {
    let (tx2, _rx) = channel();
    tx.send(tx2).unwrap();
}

pub fn main() {
    let (tx, rx) = channel();
    let child = thread::spawn(move|| {
        start(&tx)
    });
    let _tx = rx.recv().unwrap();
    child.join();
}
