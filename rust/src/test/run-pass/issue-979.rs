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

struct r<'a> {
    b: &'a Cell<isize>,
}

impl<'a> Drop for r<'a> {
    fn drop(&mut self) {
        self.b.set(self.b.get() + 1);
    }
}

fn r(b: &Cell<isize>) -> r {
    r {
        b: b
    }
}

pub fn main() {
    let b = &Cell::new(0);
    {
        let _p = Some(r(b));
    }

    assert_eq!(b.get(), 1);
}
