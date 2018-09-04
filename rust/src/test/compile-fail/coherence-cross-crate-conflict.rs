// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// The error here is strictly due to orphan rules; the impl here
// generalizes the one upstream

// aux-build:trait_impl_conflict.rs
extern crate trait_impl_conflict;
use trait_impl_conflict::Foo;

impl<A> Foo for A {
    //~^ ERROR type parameter `A` must be used as the type parameter for some local type
    //~| ERROR conflicting implementations of trait `trait_impl_conflict::Foo` for type `isize`
}

fn main() {
}
