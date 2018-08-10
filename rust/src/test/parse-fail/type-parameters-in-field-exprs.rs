// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z parse-only -Z continue-parse-after-error

struct Foo {
    x: isize,
    y: isize,
}

fn main() {
    let f = Foo {
        x: 1,
        y: 2,
    };
    f.x::<isize>;
    //~^ ERROR field expressions may not have generic arguments
    f.x::<>;
    //~^ ERROR field expressions may not have generic arguments
    f.x::();
    //~^ ERROR field expressions may not have generic arguments
}
