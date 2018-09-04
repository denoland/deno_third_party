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

fn main() {
    let foo =
        match
        Some(4).unwrap_or_else(5)
        //~^ NOTE expected one of `.`, `?`, `{`, or an operator here
        ; //~ NOTE unexpected token
        //~^ ERROR expected one of `.`, `?`, `{`, or an operator, found `;`

    println!("{}", foo)
}
