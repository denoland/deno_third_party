// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#![allow(unknown_features)]
#![feature(box_syntax)]

trait Foo<T> {
    fn get(&self) -> T;
}

struct S {
    x: isize
}

impl Foo<isize> for S {
    fn get(&self) -> isize {
        self.x
    }
}

pub fn main() {
    let x = box S { x: 1 };
    let y = x as Box<Foo<isize>>;
    assert_eq!(y.get(), 1);
}
