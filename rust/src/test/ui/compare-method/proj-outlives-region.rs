// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we elaborate `Type: 'region` constraints and infer various important things.

trait Master<'a, T: ?Sized, U> {
    fn foo() where T: 'a;
}

// `U::Item: 'a` does not imply that `U: 'a`
impl<'a, U: Iterator> Master<'a, U::Item, U> for () {
    fn foo() where U: 'a { } //~ ERROR E0276
}

fn main() {
    println!("Hello, world!");
}
