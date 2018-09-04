// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:crateresolve1-1.rs
// aux-build:crateresolve1-2.rs
// aux-build:crateresolve1-3.rs
// error-pattern:multiple matching crates for `crateresolve1`

extern crate crateresolve1;

fn main() {
}
