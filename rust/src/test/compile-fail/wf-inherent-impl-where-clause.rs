// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we check where-clauses on inherent impls.

#![feature(associated_type_defaults)]
#![feature(rustc_attrs)]
#![allow(dead_code)]

trait ExtraCopy<T:Copy> { }

struct Foo<T,U>(T,U);

impl<T,U> Foo<T,U> where T: ExtraCopy<U> //~ ERROR E0277
{
}

#[rustc_error]
fn main() { }
