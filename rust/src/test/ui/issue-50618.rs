// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct Point {
    pub x: u64,
    pub y: u64,
}

const TEMPLATE: Point = Point {
    x: 0,
    y: 0
};

fn main() {
    let _ = || {
        Point {
            nonexistent: 0,
            //~^ ERROR struct `Point` has no field named `nonexistent`
            ..TEMPLATE
        }
    };
}
