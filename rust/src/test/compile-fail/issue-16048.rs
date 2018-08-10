// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait NoLifetime {
    fn get<'p, T : Test<'p>>(&self) -> T;
    //~^ NOTE lifetimes in impl do not match this method in trait
}

trait Test<'p> {
    fn new(buf: &'p mut [u8]) -> Self;
}

struct Foo<'a> {
    buf: &'a mut [u8],
}

impl<'a> Test<'a> for Foo<'a> {
    fn new(buf: &'a mut [u8]) -> Foo<'a> {
        Foo { buf: buf }
    }
}

impl<'a> NoLifetime for Foo<'a> {
    fn get<'p, T : Test<'a>>(&self) -> T {
    //~^ ERROR E0195
    //~| NOTE lifetimes do not match method in trait
        return *self as T;
    }
}

fn main() {}
