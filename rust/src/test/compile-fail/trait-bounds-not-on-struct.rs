// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(bare_trait_objects)]

struct Foo;

fn foo(_x: Box<Foo + Send>) { } //~ ERROR expected trait, found struct `Foo`

type A<T> = Box<dyn Vec<T>>; //~ ERROR expected trait, found struct `Vec`

fn main() { }
