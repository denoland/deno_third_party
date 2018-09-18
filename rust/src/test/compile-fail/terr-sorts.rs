// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


struct foo {
    a: isize,
    b: isize,
}

type bar = Box<foo>;

fn want_foo(f: foo) {}
fn have_bar(b: bar) {
    want_foo(b); //~  ERROR mismatched types
                 //~| expected type `foo`
                 //~| found type `std::boxed::Box<foo>`
}

fn main() {}
