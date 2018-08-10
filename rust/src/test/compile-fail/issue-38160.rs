// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(associated_consts, rustc_attrs)]
#![allow(warnings)]

trait MyTrait {
    const MY_CONST: &'static str;
}

macro_rules! my_macro {
    () => {
        struct MyStruct;

        impl MyTrait for MyStruct {
            const MY_CONST: &'static str = stringify!(abc);
        }
    }
}

my_macro!();

#[rustc_error]
fn main() {} //~ ERROR compilation successful
