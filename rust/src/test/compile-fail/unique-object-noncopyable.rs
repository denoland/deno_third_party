// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(box_syntax)]

trait Foo {
    fn f(&self);
}

struct Bar {
    x: isize,
}

impl Drop for Bar {
    fn drop(&mut self) {}
}

impl Foo for Bar {
    fn f(&self) {
        println!("hi");
    }
}

fn main() {
    let x = box Bar { x: 10 };
    let y: Box<Foo> = x as Box<Foo>;
    let _z = y.clone(); //~ ERROR no method named `clone` found
}
