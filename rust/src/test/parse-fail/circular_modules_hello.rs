// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z parse-only

// ignore-test: this is an auxiliary file for circular-modules-main.rs

#[path = "circular_modules_main.rs"]
mod circular_modules_main;

pub fn say_hello() {
    println!("{}", circular_modules_main::hi_str());
}
