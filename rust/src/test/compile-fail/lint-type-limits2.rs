// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

// compile-flags: -D unused-comparisons
fn main() { }


fn bar() -> i8 {
    return 123;
}

fn baz() -> bool {
    128 > bar() //~ ERROR comparison is useless due to type limits
                //~| WARN literal out of range for i8
}
