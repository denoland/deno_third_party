// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.





enum color {
    rgb(isize, isize, isize),
    rgba(isize, isize, isize, isize),
    hsl(isize, isize, isize),
}

fn process(c: color) -> isize {
    let mut x: isize;
    match c {
      color::rgb(r, _, _) => { x = r; }
      color::rgba(_, _, _, a) => { x = a; }
      color::hsl(_, s, _) => { x = s; }
    }
    return x;
}

pub fn main() {
    let gray: color = color::rgb(127, 127, 127);
    let clear: color = color::rgba(50, 150, 250, 0);
    let red: color = color::hsl(0, 255, 255);
    assert_eq!(process(gray), 127);
    assert_eq!(process(clear), 0);
    assert_eq!(process(red), 255);
}
