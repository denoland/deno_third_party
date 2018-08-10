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

use std::sync::mpsc::{channel, Sender};
use std::thread;

struct complainer {
    tx: Sender<bool>,
}

impl Drop for complainer {
    fn drop(&mut self) {
        println!("About to send!");
        self.tx.send(true).unwrap();
        println!("Sent!");
    }
}

fn complainer(tx: Sender<bool>) -> complainer {
    println!("Hello!");
    complainer {
        tx: tx
    }
}

fn f(tx: Sender<bool>) {
    let _tx = complainer(tx);
    panic!();
}

pub fn main() {
    let (tx, rx) = channel();
    let t = thread::spawn(move|| f(tx.clone()));
    println!("hiiiiiiiii");
    assert!(rx.recv().unwrap());
    drop(t.join());
}
