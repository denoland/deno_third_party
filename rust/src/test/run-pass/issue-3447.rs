// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unknown_features)]
#![feature(box_syntax)]

use std::cell::RefCell;

static S: &'static str = "str";

struct list<T> {
    element: T,
    next: Option<Box<RefCell<list<T>>>>
}

impl<T:'static> list<T> {
    pub fn addEnd(&mut self, element: T) {
        let newList = list {
            element: element,
            next: None
        };

        self.next = Some(box RefCell::new(newList));
    }
}

pub fn main() {
    let ls = list {
        element: S,
        next: None
    };
    println!("{}", ls.element);
}
