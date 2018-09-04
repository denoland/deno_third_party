// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::mem::swap;

pub fn main() {
    let mut x = 4;

    for i in 0_usize..3 {
        // ensure that the borrow in this alt
        // does not interfere with the swap
        // below.  note that it would it you
        // naively borrowed &x for the lifetime
        // of the variable x, as we once did
        match i {
            i => {
                let y = &x;
                assert!(i < *y);
            }
        }
        let mut y = 4;
        swap(&mut y, &mut x);
    }
}
