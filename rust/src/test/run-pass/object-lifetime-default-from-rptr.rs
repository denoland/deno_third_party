// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that the lifetime of the enclosing `&` is used for the object
// lifetime bound.

// pretty-expanded FIXME #23616

#![allow(dead_code)]

use std::fmt::Display;

trait Test {
    fn foo(&self) { }
}

struct SomeStruct<'a> {
    t: &'a Test,
    u: &'a (Test+'a),
}

fn a<'a>(t: &'a Test, mut ss: SomeStruct<'a>) {
    ss.t = t;
}

fn b<'a>(t: &'a Test, mut ss: SomeStruct<'a>) {
    ss.u = t;
}

fn c<'a>(t: &'a (Test+'a), mut ss: SomeStruct<'a>) {
    ss.t = t;
}

fn d<'a>(t: &'a (Test+'a), mut ss: SomeStruct<'a>) {
    ss.u = t;
}

fn e<'a>(_: &'a (Display+'static)) {}

fn main() {
    // Inside a function body, we can just infer both
    // lifetimes, to allow &'tmp (Display+'static).
    e(&0 as &Display);
}
