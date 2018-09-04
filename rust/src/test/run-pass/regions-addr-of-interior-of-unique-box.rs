// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// pretty-expanded FIXME #23616

struct Point {
    x: isize,
    y: isize
}

struct Character {
    pos: Box<Point>,
}

fn get_x(x: &Character) -> &isize {
    // interesting case because the scope of this
    // borrow of the unique pointer is in fact
    // larger than the fn itself
    return &x.pos.x;
}

pub fn main() {
}
