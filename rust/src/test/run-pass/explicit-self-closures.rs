// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test to make sure that explicit self params work inside closures

// pretty-expanded FIXME #23616

struct Box {
    x: usize
}

impl Box {
    pub fn set_many(&mut self, xs: &[usize]) {
        for x in xs { self.x = *x; }
    }
}

pub fn main() {}
