// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:cci_class_cast.rs

#![allow(unknown_features)]
#![feature(box_syntax)]

extern crate cci_class_cast;

use std::string::ToString;
use cci_class_cast::kitty::cat;

fn print_out(thing: Box<ToString>, expected: String) {
  let actual = (*thing).to_string();
  println!("{}", actual);
  assert_eq!(actual.to_string(), expected);
}

pub fn main() {
  let nyan: Box<ToString> = box cat(0, 2, "nyan".to_string()) as Box<ToString>;
  print_out(nyan, "nyan".to_string());
}
