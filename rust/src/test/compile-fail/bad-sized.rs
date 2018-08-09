// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait Trait {}

pub fn main() {
    let x: Vec<Trait + Sized> = Vec::new();
    //~^ ERROR only auto traits can be used as additional traits in a trait object
    //~| ERROR the trait bound `Trait: std::marker::Sized` is not satisfied
    //~| ERROR the trait bound `Trait: std::marker::Sized` is not satisfied
}
