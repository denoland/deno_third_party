// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:xcrate.rs
// compile-flags: --edition=2018 -Zunstable-options

#![feature(crate_in_paths)]
#![feature(extern_absolute_paths)]

use crate; //~ ERROR unresolved import `crate`
           //~^ NOTE crate root imports need to be explicitly named: `use crate as name;`
use *; //~ ERROR unresolved import `*`
       //~^ NOTE cannot glob-import all possible crates

fn main() {
    let s = ::xcrate; //~ ERROR expected value, found module `xcrate`
                      //~^ NOTE not a value
}
