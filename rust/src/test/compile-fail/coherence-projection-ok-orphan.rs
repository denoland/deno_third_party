// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(rustc_attrs)]
#![allow(dead_code)]

// Here we do not get a coherence conflict because `Baz: Iterator`
// does not hold and (due to the orphan rules), we can rely on that.

pub trait Foo<P> {}

pub trait Bar {
    type Output: 'static;
}

struct Baz;
impl Foo<i32> for Baz { }

impl<A:Iterator> Foo<A::Item> for A { }

#[rustc_error]
fn main() {} //~ ERROR compilation successful
