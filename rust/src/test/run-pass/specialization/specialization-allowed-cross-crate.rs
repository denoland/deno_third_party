// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:go_trait.rs

#![feature(specialization)]

extern crate go_trait;

use go_trait::{Go,GoMut};
use std::fmt::Debug;
use std::default::Default;

struct MyThingy;

impl Go for MyThingy {
    fn go(&self, arg: isize) { }
}

impl GoMut for MyThingy {
    fn go_mut(&mut self, arg: isize) { }
}

fn main() { }
