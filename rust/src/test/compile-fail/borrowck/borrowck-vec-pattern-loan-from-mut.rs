// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(slice_patterns)]

fn a() {
    let mut v = vec![1, 2, 3];
    let vb: &mut [isize] = &mut v;
    match vb {
        &mut [_a, ref tail..] => {
            v.push(tail[0] + tail[1]); //~ ERROR cannot borrow
        }
        _ => {}
    };
}

fn main() {}
