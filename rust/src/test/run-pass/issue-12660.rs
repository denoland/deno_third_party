// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:issue-12660-aux.rs

// pretty-expanded FIXME #23616

extern crate issue12660aux;

use issue12660aux::{my_fn, MyStruct};

#[allow(path_statements)]
fn main() {
    my_fn(MyStruct);
    MyStruct;
}
