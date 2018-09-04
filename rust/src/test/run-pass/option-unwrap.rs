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

struct dtor<'a> {
    x: &'a Cell<isize>,
}

impl<'a> Drop for dtor<'a> {
    fn drop(&mut self) {
        self.x.set(self.x.get() - 1);
    }
}

fn unwrap<T>(o: Option<T>) -> T {
    match o {
      Some(v) => v,
      None => panic!()
    }
}

pub fn main() {
    let x = &Cell::new(1);

    {
        let b = Some(dtor { x:x });
        let _c = unwrap(b);
    }

    assert_eq!(x.get(), 0);
}
