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

#[derive(Debug)]
struct r<'a> {
    i: &'a Cell<isize>,
}

impl<'a> Drop for r<'a> {
    fn drop(&mut self) {
        self.i.set(self.i.get() + 1);
    }
}

fn r(i: &Cell<isize>) -> r {
    r {
        i: i
    }
}

pub fn main() {
    let i = &Cell::new(0);
    // Even though these look like copies, they are guaranteed not to be
    {
        let a = r(i);
        let b = (a, 10);
        let (c, _d) = b;
        println!("{:?}", c);
    }
    assert_eq!(i.get(), 1);
}
