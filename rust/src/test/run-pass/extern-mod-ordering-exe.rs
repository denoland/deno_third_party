// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:extern_mod_ordering_lib.rs

// pretty-expanded FIXME #23616

extern crate extern_mod_ordering_lib;

use extern_mod_ordering_lib::extern_mod_ordering_lib as the_lib;

pub fn main() {
    the_lib::f();
}
