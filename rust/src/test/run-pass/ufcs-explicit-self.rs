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

#[derive(Copy, Clone)]
struct Foo {
    f: isize,
}

impl Foo {
    fn foo(self: Foo, x: isize) -> isize {
        self.f + x
    }
    fn bar(self: &Foo, x: isize) -> isize {
        self.f + x
    }
    fn baz(self: Box<Foo>, x: isize) -> isize {
        self.f + x
    }
}

#[derive(Copy, Clone)]
struct Bar<T> {
    f: T,
}

impl<T> Bar<T> {
    fn foo(self: Bar<T>, x: isize) -> isize {
        x
    }
    fn bar<'a>(self: &'a Bar<T>, x: isize) -> isize {
        x
    }
    fn baz(self: Bar<T>, x: isize) -> isize {
        x
    }
}

fn main() {
    let foo: Box<_> = box Foo {
        f: 1,
    };
    println!("{} {} {}", foo.foo(2), foo.bar(2), foo.baz(2));
    let bar: Box<_> = box Bar {
        f: 1,
    };
    println!("{} {} {}", bar.foo(2), bar.bar(2), bar.baz(2));
    let bar: Box<Bar<isize>> = bar;
    println!("{} {} {}", bar.foo(2), bar.bar(2), bar.baz(2));
}
