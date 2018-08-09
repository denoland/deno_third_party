// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// gate-test-box_syntax

// Check that `box EXPR` is feature-gated.
//
// See also feature-gate-placement-expr.rs
//
// (Note that the two tests are separated since the checks appear to
// be performed at distinct phases, with an abort_if_errors call
// separating them.)

fn main() {
    let x = box 'c'; //~ ERROR box expression syntax is experimental
    println!("x: {}", x);
}
