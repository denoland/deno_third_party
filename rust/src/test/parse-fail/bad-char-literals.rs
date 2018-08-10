// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z parse-only -Z continue-parse-after-error

// ignore-tidy-cr
// ignore-tidy-tab
fn main() {
    // these literals are just silly.
    ''';
    //~^ ERROR: character constant must be escaped: '

    // note that this is a literal "\n" byte
    '
';
    //~^^ ERROR: character constant must be escaped: \n

    // note that this is a literal "\r" byte
    ''; //~ ERROR: character constant must be escaped: \r

    // note that this is a literal tab character here
    '	';
    //~^ ERROR: character constant must be escaped: \t
}
