// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct Leak<'a> {
    dropped: &'a mut bool
}

impl<'a> Drop for Leak<'a> {
    fn drop(&mut self) {
        *self.dropped = true;
    }
}

fn main() {
    let mut dropped = false;
    {
        let leak = Leak { dropped: &mut dropped };
        for ((), leaked) in Some(((), leak)).into_iter() {}
    }

    assert!(dropped);
}
