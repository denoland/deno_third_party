// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(PartialEq, Debug)]
pub struct Partial<T> { x: T, y: T }

#[derive(PartialEq, Debug)]
struct S { val: isize }
impl S { fn new(v: isize) -> S { S { val: v } } }
impl Drop for S { fn drop(&mut self) { } }

pub fn f<T, F>((b1, b2): (T, T), mut f: F) -> Partial<T> where F: FnMut(T) -> T {
    let p = Partial { x: b1, y: b2 };

    // Move of `p` is legal even though we are also moving `p.y`; the
    // `..p` moves all fields *except* `p.y` in this context.
    Partial { y: f(p.y), ..p }
}

pub fn main() {
    let p = f((S::new(3), S::new(4)), |S { val: z }| S::new(z+1));
    assert_eq!(p, Partial { x: S::new(3), y: S::new(5) });
}
