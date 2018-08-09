// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



fn range_<F>(a: isize, b: isize, mut it: F) where F: FnMut(isize) {
    assert!((a < b));
    let mut i: isize = a;
    while i < b { it(i); i += 1; }
}

pub fn main() {
    let mut sum: isize = 0;
    range_(0, 100, |x| sum += x );
    println!("{}", sum);
}
