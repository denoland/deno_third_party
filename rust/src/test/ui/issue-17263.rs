// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(box_syntax, rustc_attrs)]

struct Foo { a: isize, b: isize }

fn main() { #![rustc_error] // rust-lang/rust#49855
    let mut x: Box<_> = box Foo { a: 1, b: 2 };
    let (a, b) = (&mut x.a, &mut x.b);
    //~^ ERROR cannot borrow `x` (via `x.b`) as mutable more than once at a time

    let mut foo: Box<_> = box Foo { a: 1, b: 2 };
    let (c, d) = (&mut foo.a, &foo.b);
    //~^ ERROR cannot borrow `foo` (via `foo.b`) as immutable
}
