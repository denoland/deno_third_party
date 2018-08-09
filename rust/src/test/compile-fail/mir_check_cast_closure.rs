// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z borrowck=mir

#![allow(dead_code)]

fn bar<'a, 'b>() -> fn(&'a u32, &'b u32) -> &'a u32 {
    let g: fn(_, _) -> _ = |_x, y| y;
    //~^ ERROR free region `'b` does not outlive free region `'a`
    g
    //~^ WARNING not reporting region error due to nll
}

fn main() {}
