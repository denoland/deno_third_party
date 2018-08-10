// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// (typeof used because it's surprisingly hard to find an unparsed token after a stmt)
macro_rules! m {
    () => ( i ; typeof );   //~ ERROR expected expression, found reserved keyword `typeof`
                            //~| ERROR macro expansion ignores token `typeof`
                            //~| ERROR macro expansion ignores token `;`
                            //~| ERROR macro expansion ignores token `;`
}

fn main() {
    let a: m!();
    let i = m!();
    match 0 {
        m!() => {}
    }

    m!();
}
