// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z parse-only

fn f<T>() {}

fn main() {
    false == false == false;
    //~^ ERROR: chained comparison operators require parentheses

    false == 0 < 2;
    //~^ ERROR: chained comparison operators require parentheses

    f<X>();
    //~^ ERROR: chained comparison operators require parentheses
    //~| HELP: use `::<...>` instead of `<...>`
    //~| HELP: or use `(...)`
}
