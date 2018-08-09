// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// This test checks that genuine type errors with partial
// type hints are understandable.

use std::marker::PhantomData;

struct Foo<T>(PhantomData<T>);
struct Bar<U>(PhantomData<U>);

pub fn main() {
}

fn test1() {
    let x: Foo<_> = Bar::<usize>(PhantomData);
    //~^ ERROR mismatched types
    //~| expected type `Foo<_>`
    //~| found type `Bar<usize>`
    //~| expected struct `Foo`, found struct `Bar`
    let y: Foo<usize> = x;
}

fn test2() {
    let x: Foo<_> = Bar::<usize>(PhantomData);
    //~^ ERROR mismatched types
    //~| expected type `Foo<_>`
    //~| found type `Bar<usize>`
    //~| expected struct `Foo`, found struct `Bar`
}
