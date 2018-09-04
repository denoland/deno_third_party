// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait Product {
    fn product(&self) -> isize;
}

struct Foo {
    x: isize,
    y: isize,
}

impl Foo {
    pub fn sum(&self) -> isize {
        self.x + self.y
    }
}

impl Product for Foo {
    fn product(&self) -> isize {
        self.x * self.y
    }
}

fn Foo(x: isize, y: isize) -> Foo {
    Foo { x: x, y: y }
}

pub fn main() {
    let foo = Foo(3, 20);
    println!("{} {}", foo.sum(), foo.product());
}
