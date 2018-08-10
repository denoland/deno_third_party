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

pub struct Entry<A,B> {
    key: A,
    value: B
}

pub struct alist<A,B> {
    eq_fn: extern "Rust" fn(A,A) -> bool,
    data: Box<RefCell<Vec<Entry<A,B>>>>,
}

pub fn alist_add<A:'static,B:'static>(lst: &alist<A,B>, k: A, v: B) {
    let mut data = lst.data.borrow_mut();
    (*data).push(Entry{key:k, value:v});
}

pub fn alist_get<A:Clone + 'static,
                 B:Clone + 'static>(
                 lst: &alist<A,B>,
                 k: A)
                 -> B {
    let eq_fn = lst.eq_fn;
    let data = lst.data.borrow();
    for entry in &(*data) {
        if eq_fn(entry.key.clone(), k.clone()) {
            return entry.value.clone();
        }
    }
    panic!();
}

#[inline]
pub fn new_int_alist<B:'static>() -> alist<isize, B> {
    fn eq_int(a: isize, b: isize) -> bool { a == b }
    return alist {
        eq_fn: eq_int,
        data: box RefCell::new(Vec::new()),
    };
}

#[inline]
pub fn new_int_alist_2<B:'static>() -> alist<isize, B> {
    #[inline]
    fn eq_int(a: isize, b: isize) -> bool { a == b }
    return alist {
        eq_fn: eq_int,
        data: box RefCell::new(Vec::new()),
    };
}
