// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:issue_2316_a.rs
// aux-build:issue_2316_b.rs

// pretty-expanded FIXME #23616

extern crate issue_2316_b;
use issue_2316_b::cloth;

pub fn main() {
  let _c: cloth::fabric = cloth::fabric::calico;
}
