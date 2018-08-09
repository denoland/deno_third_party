// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::marker;

fn send<T:Send + std::fmt::Debug>(ch: _chan<T>, data: T) {
    println!("{:?}", ch);
    println!("{:?}", data);
    panic!();
}

#[derive(Debug)]
struct _chan<T>(isize, marker::PhantomData<T>);

// Tests that "log(debug, message);" is flagged as using
// message after the send deinitializes it
fn test00_start(ch: _chan<Box<isize>>, message: Box<isize>, _count: Box<isize>) {
    send(ch, message);
    println!("{}", message); //~ ERROR use of moved value: `message`
}

fn main() { panic!(); }
