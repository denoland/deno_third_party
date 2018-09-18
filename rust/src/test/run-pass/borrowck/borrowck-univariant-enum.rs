// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



use std::cell::Cell;

#[derive(Copy, Clone)]
enum newtype {
    newvar(isize)
}

pub fn main() {

    // Test that borrowck treats enums with a single variant
    // specially.

    let x = &Cell::new(5);
    let y = &Cell::new(newtype::newvar(3));
    let z = match y.get() {
      newtype::newvar(b) => {
        x.set(x.get() + 1);
        x.get() * b
      }
    };
    assert_eq!(z, 18);
}
