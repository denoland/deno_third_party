// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


fn takes_two(x: &isize, y: &isize) -> isize { *x + *y }

fn with<T, F>(f: F) -> T where F: FnOnce(&isize) -> T {
    f(&20)
}

fn has_one<'a>(x: &'a isize) -> isize {
    with(|y| takes_two(x, y))
}

pub fn main() {
    assert_eq!(has_one(&2), 22);
}
