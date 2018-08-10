// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


enum E { V, VV(isize) }
static C: E = E::V;

impl E {
    pub fn method(&self) {
        match *self {
            E::V => {}
            E::VV(..) => panic!()
        }
    }
}

pub fn main() {
    C.method()
}
