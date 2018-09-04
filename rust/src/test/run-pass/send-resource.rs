// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
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

use std::thread;
use std::sync::mpsc::channel;

struct test {
  f: isize,
}

impl Drop for test {
    fn drop(&mut self) {}
}

fn test(f: isize) -> test {
    test {
        f: f
    }
}

pub fn main() {
    let (tx, rx) = channel();

    let t = thread::spawn(move|| {
        let (tx2, rx2) = channel();
        tx.send(tx2).unwrap();

        let _r = rx2.recv().unwrap();
    });

    rx.recv().unwrap().send(test(42)).unwrap();

    t.join();
}
