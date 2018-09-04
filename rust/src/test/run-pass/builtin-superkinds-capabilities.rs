// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Tests "capabilities" granted by traits that inherit from super-
// builtin-kinds, e.g., if a trait requires Send to implement, then
// at usage site of that trait, we know we have the Send capability.


use std::sync::mpsc::{channel, Sender, Receiver};

trait Foo : Send { }

impl <T: Send> Foo for T { }

fn foo<T: Foo + 'static>(val: T, chan: Sender<T>) {
    chan.send(val).unwrap();
}

pub fn main() {
    let (tx, rx): (Sender<isize>, Receiver<isize>) = channel();
    foo(31337, tx);
    assert_eq!(rx.recv().unwrap(), 31337);
}
