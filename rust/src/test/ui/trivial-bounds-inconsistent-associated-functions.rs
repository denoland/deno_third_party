// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// run-pass
// Inconsistent bounds with trait implementations

#![feature(trivial_bounds)]
#![allow(unused)]

trait A {
    fn foo(&self) -> Self where Self: Copy;
}

impl A for str {
    fn foo(&self) -> Self where Self: Copy { *"" }
}

impl A for i32 {
    fn foo(&self) -> Self { 3 }
}

fn main() {}
