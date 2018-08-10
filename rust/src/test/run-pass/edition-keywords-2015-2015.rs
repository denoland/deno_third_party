// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: --edition=2015
// aux-build:edition-kw-macro-2015.rs

#![feature(raw_identifiers)]

#[macro_use]
extern crate edition_kw_macro_2015;

pub fn check_async() {
    let mut async = 1; // OK
    let mut r#async = 1; // OK

    r#async = consumes_async!(async); // OK
    // r#async = consumes_async!(r#async); // ERROR, not a match
    // r#async = consumes_async_raw!(async); // ERROR, not a match
    r#async = consumes_async_raw!(r#async); // OK

    if passes_ident!(async) == 1 {} // OK
    if passes_ident!(r#async) == 1 {} // OK
    one_async::async(); // OK
    one_async::r#async(); // OK
    two_async::async(); // OK
    two_async::r#async(); // OK
}

mod one_async {
    produces_async! {} // OK
}
mod two_async {
    produces_async_raw! {} // OK
}

fn main() {}
