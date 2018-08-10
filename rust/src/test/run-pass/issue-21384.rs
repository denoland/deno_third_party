// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use ::std::ops::RangeFull;

fn test<T : Clone>(arg: T) -> T {
    arg.clone()
}

#[derive(PartialEq, Debug)]
struct Test(isize);

fn main() {
    // Check that ranges implement clone
    assert_eq!(test(1..5), (1..5));
    assert_eq!(test(..5), (..5));
    assert_eq!(test(1..), (1..));
    assert_eq!(test(RangeFull), (RangeFull));

    // Check that ranges can still be used with non-clone limits
    assert_eq!((Test(1)..Test(5)), (Test(1)..Test(5)));
}
