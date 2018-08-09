// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that you can insert an explicit lifetime in explicit self.

// pretty-expanded FIXME #23616

struct Foo {
    f: usize
}

impl Foo {
    pub fn foo<'a>(&'a self) {}
}

pub fn main() {}
