// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Attempt to extend the lifetime as well as unsizing.

#![feature(unsized_tuple_coercion)]

struct Fat<T: ?Sized> {
    ptr: T
}

struct Foo;
trait Bar { fn bar(&self) {} }
impl Bar for Foo {}

fn baz<'a>() {
    // With a vec of ints.
    let f1 = Fat { ptr: [1, 2, 3] };
    let f2: &Fat<[isize; 3]> = &f1; //~ ERROR `f1` does not live long enough
    let f3: &'a Fat<[isize]> = f2;

    // With a trait.
    let f1 = Fat { ptr: Foo };
    let f2: &Fat<Foo> = &f1; //~ ERROR `f1` does not live long enough
    let f3: &'a Fat<Bar> = f2;

    // Tuple with a vec of ints.
    let f1 = ([1, 2, 3],);
    let f2: &([isize; 3],) = &f1; //~ ERROR `f1` does not live long enough
    let f3: &'a ([isize],) = f2;

    // Tuple with a trait.
    let f1 = (Foo,);
    let f2: &(Foo,) = &f1; //~ ERROR `f1` does not live long enough
    let f3: &'a (Bar,) = f2;
}

pub fn main() {
    baz();
}
