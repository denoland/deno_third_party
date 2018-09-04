// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:attr-args.rs
// ignore-stage1

#![allow(warnings)]
#![feature(proc_macro, proc_macro_path_invoc)]

extern crate attr_args;
use attr_args::attr_with_args;

#[attr_with_args(text = "Hello, world!")]
fn foo() {}

#[::attr_args::identity(
  fn main() { assert_eq!(foo(), "Hello, world!"); })]
struct Dummy;
