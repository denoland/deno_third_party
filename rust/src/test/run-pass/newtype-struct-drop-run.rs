// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Make sure the destructor is run for newtype structs.

use std::cell::Cell;

struct Foo<'a>(&'a Cell<isize>);

impl<'a> Drop for Foo<'a> {
    fn drop(&mut self) {
        let Foo(i) = *self;
        i.set(23);
    }
}

pub fn main() {
    let y = &Cell::new(32);
    {
        let _x = Foo(y);
    }
    assert_eq!(y.get(), 23);
}
