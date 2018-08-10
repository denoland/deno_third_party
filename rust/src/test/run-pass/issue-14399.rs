// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// #14399
// We'd previously ICE if we had a method call whose return
// value was coerced to a trait object. (v.clone() returns Box<B1>
// which is coerced to Box<A>).

// pretty-expanded FIXME #23616

#![allow(unknown_features)]
#![feature(box_syntax)]

#[derive(Clone)]
struct B1;

trait A { fn foo(&self) {} }
impl A for B1 {}

fn main() {
    let v: Box<_> = box B1;
    let _c: Box<A> = v.clone();
}
