// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we are able to infer a suitable kind for this closure
// that is just called (`FnMut`).

fn main() {
    let mut counter = 0;

    // Here this must be inferred to FnMut so that it can mutate counter,
    // but we forgot the mut.
    let tick1 = || {
        counter += 1;
    };

    // In turn, tick2 must be inferred to FnMut so that it can call
    // tick1, but we forgot the mut. The error message we currently
    // get seems... suboptimal.
    let tick2 = || { //~ ERROR closure cannot assign to immutable local variable `tick1`
        tick1();
    };

    tick2(); //~ ERROR cannot borrow
}
