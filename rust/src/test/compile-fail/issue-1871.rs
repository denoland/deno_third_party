// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Tests that we don't generate a spurious error about f.honk's type
// being undeterminable
fn main() {
  let f = 42;

  let _g = if f < 5 {
      f.honk() //~ ERROR no method named `honk` found
  }
  else {
      ()
  };
}
