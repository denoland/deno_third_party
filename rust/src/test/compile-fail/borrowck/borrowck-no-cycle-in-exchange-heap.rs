// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(box_syntax)]

struct node_ {
    a: Box<cycle>
}

enum cycle {
    node(node_),
    empty
}
fn main() {
    let mut x: Box<_> = box cycle::node(node_ {a: box cycle::empty});
    // Create a cycle!
    match *x {
      cycle::node(ref mut y) => {
        y.a = x; //~ ERROR cannot move out of
      }
      cycle::empty => {}
    };
}
