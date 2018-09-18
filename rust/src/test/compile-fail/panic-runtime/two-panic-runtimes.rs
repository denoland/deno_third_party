// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern:cannot link together two panic runtimes: panic_runtime_unwind and panic_runtime_unwind2
// ignore-tidy-linelength
// aux-build:panic-runtime-unwind.rs
// aux-build:panic-runtime-unwind2.rs
// aux-build:panic-runtime-lang-items.rs

#![no_std]

extern crate panic_runtime_unwind;
extern crate panic_runtime_unwind2;
extern crate panic_runtime_lang_items;

fn main() {}
