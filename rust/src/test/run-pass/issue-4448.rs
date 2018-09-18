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

use std::sync::mpsc::channel;
use std::thread;

pub fn main() {
    let (tx, rx) = channel::<&'static str>();

    let t = thread::spawn(move|| {
        assert_eq!(rx.recv().unwrap(), "hello, world");
    });

    tx.send("hello, world").unwrap();
    t.join().ok().unwrap();
}
