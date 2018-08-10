// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test the uninit() construct returning various empty types.

// pretty-expanded FIXME #23616

use std::mem;

#[derive(Clone)]
struct Foo;

pub fn main() {
    unsafe {
        let _x: Foo = mem::uninitialized();
        let _x: [Foo; 2] = mem::uninitialized();
    }
}
