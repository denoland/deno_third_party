// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// aux-build:trait_superkinds_in_metadata.rs

// Tests "capabilities" granted by traits with super-builtin-kinds,
// even when using them cross-crate.


extern crate trait_superkinds_in_metadata;

use std::sync::mpsc::{channel, Sender, Receiver};
use trait_superkinds_in_metadata::{RequiresRequiresShareAndSend, RequiresShare};

#[derive(PartialEq, Debug)]
struct X<T>(T);

impl <T: Sync> RequiresShare for X<T> { }
impl <T: Sync+Send> RequiresRequiresShareAndSend for X<T> { }

fn foo<T: RequiresRequiresShareAndSend + 'static>(val: T, chan: Sender<T>) {
    chan.send(val).unwrap();
}

pub fn main() {
    let (tx, rx): (Sender<X<isize>>, Receiver<X<isize>>) = channel();
    foo(X(31337), tx);
    assert_eq!(rx.recv().unwrap(), X(31337));
}
