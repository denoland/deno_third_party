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

#![allow(unknown_features)]
#![feature(box_syntax)]

pub struct Foo {
    a: isize,
}

struct Bar<'a> {
    a: Box<Option<isize>>,
    b: &'a Foo,
}

fn check(a: Box<Foo>) {
    let _ic = Bar{ b: &*a, a: box None };
}

pub fn main(){}
