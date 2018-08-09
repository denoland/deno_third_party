// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

use std::ops::{Deref, DerefMut};

// Generic unique/owned smaht pointer.
struct Own<T> {
    value: *mut T
}

impl<T> Deref for Own<T> {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a T {
        unsafe { &*self.value }
    }
}

impl<T> DerefMut for Own<T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe { &mut *self.value }
    }
}

struct Point {
    x: isize,
    y: isize
}

impl Point {
    fn get(&mut self) -> (isize, isize) {
        (self.x, self.y)
    }
}

fn test0(mut x: Own<Point>) {
    let _ = x.get();
}

fn test1(mut x: Own<Own<Own<Point>>>) {
    let _ = x.get();
}

fn test2(mut x: Own<Own<Own<Point>>>) {
    let _ = (**x).get();
}

fn main() {}
