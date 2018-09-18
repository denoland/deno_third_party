// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(deprecated)]
#![feature(dropck_parametricity)]

struct Foo;

impl Drop for Foo {
    #[unsafe_destructor_blind_to_params]
    //~^ ERROR use of deprecated attribute `dropck_parametricity`
    fn drop(&mut self) {}
}

fn main() {}
