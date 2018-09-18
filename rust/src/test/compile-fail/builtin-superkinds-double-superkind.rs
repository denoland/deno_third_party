// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test for traits that inherit from multiple builtin kinds at once,
// testing that all such kinds must be present on implementing types.

trait Foo : Send+Sync { }

impl <T: Sync+'static> Foo for (T,) { }
//~^ ERROR the trait bound `T: std::marker::Send` is not satisfied in `(T,)` [E0277]

impl <T: Send> Foo for (T,T) { }
//~^ ERROR `T` cannot be shared between threads safely [E0277]

impl <T: Send+Sync> Foo for (T,T,T) { } // (ok)

fn main() { }
