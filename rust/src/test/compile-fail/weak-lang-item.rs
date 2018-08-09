// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:weak-lang-items.rs
// error-pattern: language item required, but not found: `panic_impl`
// error-pattern: language item required, but not found: `eh_personality`
// ignore-wasm32-bare compiled with panic=abort, personality not required

#![no_std]

extern crate core;
extern crate weak_lang_items;

fn main() {}
