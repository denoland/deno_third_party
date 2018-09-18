// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: --edition=2018
// aux-build:edition-kw-macro-2015.rs

#![feature(raw_identifiers)]

#[macro_use]
extern crate edition_kw_macro_2015;

pub fn check_async() {
    let mut async = 1; //~ ERROR expected identifier, found reserved keyword `async`
    let mut r#async = 1; // OK

    r#async = consumes_async!(async); // OK
    r#async = consumes_async!(r#async); //~ ERROR no rules expected the token `r#async`
    r#async = consumes_async_raw!(async); //~ ERROR no rules expected the token `async`
    r#async = consumes_async_raw!(r#async); // OK

    if passes_ident!(async) == 1 {} //~ ERROR expected expression, found reserved keyword `async`
    if passes_ident!(r#async) == 1 {} // OK
    module::async(); //~ ERROR expected identifier, found reserved keyword `async`
    module::r#async(); // OK
}
