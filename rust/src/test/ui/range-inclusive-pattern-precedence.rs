// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// In expression, `&a..=b` is treated as `(&a)..=(b)` and `box a..=b` is
// `(box a)..=(b)`. In a pattern, however, `&a..=b` means `&(a..=b)`. This may
// lead to confusion.
//
// We are going to disallow `&a..=b` and `box a..=b` in a pattern. However, the
// older ... syntax is still allowed as a stability guarantee.

#![feature(box_patterns)]

pub fn main() {
    match &12 {
        &0...9 => {}
        &10..=15 => {}
        //~^ ERROR the range pattern here has ambiguous interpretation
        //~^^ HELP add parentheses to clarify the precedence
        &(16..=20) => {}
        _ => {}
    }

    match Box::new(12) {
        box 0...9 => {}
        box 10..=15 => {}
        //~^ ERROR the range pattern here has ambiguous interpretation
        //~^^ HELP add parentheses to clarify the precedence
        box (16..=20) => {}
        _ => {}
    }
}
