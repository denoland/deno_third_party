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
#![feature(box_syntax)]

use std::sync::mpsc::{channel, Sender};
use std::thread;

fn child(tx: &Sender<Box<usize>>, i: usize) {
    tx.send(box i).unwrap();
}

pub fn main() {
    let (tx, rx) = channel();
    let n = 100;
    let mut expected = 0;
    let ts = (0..n).map(|i| {
        expected += i;
        let tx = tx.clone();
        thread::spawn(move|| {
            child(&tx, i)
        })
    }).collect::<Vec<_>>();

    let mut actual = 0;
    for _ in 0..n {
        let j = rx.recv().unwrap();
        actual += *j;
    }

    assert_eq!(expected, actual);

    for t in ts { t.join(); }
}
