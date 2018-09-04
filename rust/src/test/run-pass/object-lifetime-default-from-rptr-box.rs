// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that the lifetime from the enclosing `&` is "inherited"
// through the `Box` struct.

// pretty-expanded FIXME #23616

#![allow(dead_code)]

trait Test {
    fn foo(&self) { }
}

struct SomeStruct<'a> {
    t: &'a Box<Test>,
    u: &'a Box<Test+'a>,
}

fn a<'a>(t: &'a Box<Test>, mut ss: SomeStruct<'a>) {
    ss.t = t;
}

fn b<'a>(t: &'a Box<Test>, mut ss: SomeStruct<'a>) {
    ss.u = t;
}

// see also compile-fail/object-lifetime-default-from-rptr-box-error.rs

fn d<'a>(t: &'a Box<Test+'a>, mut ss: SomeStruct<'a>) {
    ss.u = t;
}

fn main() {
}
