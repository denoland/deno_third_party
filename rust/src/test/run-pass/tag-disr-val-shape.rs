// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(Debug)]
enum color {
    red = 0xff0000,
    green = 0x00ff00,
    blue = 0x0000ff,
    black = 0x000000,
    white = 0xFFFFFF,
}

pub fn main() {
    let act = format!("{:?}", color::red);
    println!("{}", act);
    assert_eq!("red".to_string(), act);
    assert_eq!("green".to_string(), format!("{:?}", color::green));
    assert_eq!("white".to_string(), format!("{:?}", color::white));
}
