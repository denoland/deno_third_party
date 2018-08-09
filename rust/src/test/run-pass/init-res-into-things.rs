// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(box_syntax)]

use std::cell::Cell;

// Resources can't be copied, but storing into data structures counts
// as a move unless the stored thing is used afterwards.

struct r<'a> {
    i: &'a Cell<isize>,
}

struct BoxR<'a> { x: r<'a> }

impl<'a> Drop for r<'a> {
    fn drop(&mut self) {
        self.i.set(self.i.get() + 1)
    }
}

fn r(i: &Cell<isize>) -> r {
    r {
        i: i
    }
}

fn test_rec() {
    let i = &Cell::new(0);
    {
        let _a = BoxR {x: r(i)};
    }
    assert_eq!(i.get(), 1);
}

fn test_tag() {
    enum t<'a> {
        t0(r<'a>),
    }

    let i = &Cell::new(0);
    {
        let _a = t::t0(r(i));
    }
    assert_eq!(i.get(), 1);
}

fn test_tup() {
    let i = &Cell::new(0);
    {
        let _a = (r(i), 0);
    }
    assert_eq!(i.get(), 1);
}

fn test_unique() {
    let i = &Cell::new(0);
    {
        let _a: Box<_> = box r(i);
    }
    assert_eq!(i.get(), 1);
}

fn test_unique_rec() {
    let i = &Cell::new(0);
    {
        let _a: Box<_> = box BoxR {
            x: r(i)
        };
    }
    assert_eq!(i.get(), 1);
}

pub fn main() {
    test_rec();
    test_tag();
    test_tup();
    test_unique();
    test_unique_rec();
}
