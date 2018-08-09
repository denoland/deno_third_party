// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-pretty pretty-printing is unhygienic

// aux-build:intercrate.rs

// error-pattern:type `fn() -> u32 {intercrate::foo::bar::f}` is private

#![feature(decl_macro)]

extern crate intercrate;

fn main() {
    assert_eq!(intercrate::foo::m!(), 1);
}
