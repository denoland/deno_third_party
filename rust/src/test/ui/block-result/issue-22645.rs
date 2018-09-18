// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::Add;

trait Scalar {}
impl Scalar for f64 {}

struct Bob;

impl<RHS: Scalar> Add <RHS> for Bob {
  type Output = Bob;
  fn add(self, rhs : RHS) -> Bob { Bob }
}

fn main() {
  let b = Bob + 3.5;
  b + 3 //~ ERROR E0277
  //~^ ERROR: mismatched types
}
