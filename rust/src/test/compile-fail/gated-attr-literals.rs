// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that literals in attributes don't parse without the feature gate.

// gate-test-attr_literals
// gate-test-custom_attribute

#![feature(rustc_attrs)]
#![allow(dead_code)]
#![allow(unused_variables)]

#[fake_attr] //~ ERROR attribute `fake_attr` is currently unknown
#[fake_attr(100)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes
#[fake_attr(1, 2, 3)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes
#[fake_attr("hello")] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR string literals in top-level positions, are experimental
#[fake_attr(name = "hello")] //~ ERROR attribute `fake_attr` is currently unknown
#[fake_attr(1, "hi", key = 12, true, false)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes, or string literals in top-level positions
#[fake_attr(key = "hello", val = 10)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes
#[fake_attr(key("hello"), val(10))] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes, or string literals in top-level positions
#[fake_attr(enabled = true, disabled = false)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes
#[fake_attr(true)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes
#[fake_attr(pi = 3.14159)] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR non-string literals in attributes
#[fake_attr(b"hi")] //~ ERROR attribute `fake_attr` is currently unknown
    //~^ ERROR string literals in top-level positions, are experimental
#[fake_doc(r"doc")] //~ ERROR attribute `fake_doc` is currently unknown
    //~^ ERROR string literals in top-level positions, are experimental
struct Q {  }

#[rustc_error]
fn main() { }
