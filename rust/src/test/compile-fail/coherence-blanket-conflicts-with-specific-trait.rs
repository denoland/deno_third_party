// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that a blank impl for all T:PartialEq conflicts with an impl for some
// specific T when T:PartialEq.

trait OtherTrait {
    fn noop(&self);
}

trait MyTrait {
    fn get(&self) -> usize;
}

impl<T:OtherTrait> MyTrait for T {
    fn get(&self) -> usize { 0 }
}

struct MyType {
    dummy: usize
}

impl MyTrait for MyType { //~ ERROR E0119
    fn get(&self) -> usize { self.dummy }
}

impl OtherTrait for MyType {
    fn noop(&self) { }
}

fn main() { }
