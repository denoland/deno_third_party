// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


trait Foo {
    const ID: usize;
}

const X: [i32; <i32 as Foo>::ID] = [0, 1, 2];
//~^ ERROR the trait bound `i32: Foo` is not satisfied

fn main() {
    assert_eq!(1, X);
}
