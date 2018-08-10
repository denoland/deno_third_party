// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

trait Foo {
}

fn b(_x: Box<Foo+Send>) {
}

fn c(x: Box<Foo+Sync+Send>) {
    e(x);
}

fn d(x: Box<Foo+Send>) {
    e(x);
}

fn e(x: Box<Foo>) {
    e(x);
}

pub fn main() { }
