// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:issue_17718_const_privacy.rs

extern crate issue_17718_const_privacy as other;

use a::B; //~ ERROR: constant `B` is private
use other::{
    FOO,
    BAR, //~ ERROR: constant `BAR` is private
    FOO2,
};

mod a {
    const B: usize = 3;
}

fn main() {}
