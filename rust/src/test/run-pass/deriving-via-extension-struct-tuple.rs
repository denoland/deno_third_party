// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(PartialEq, Debug)]
struct Foo(isize, isize, String);

pub fn main() {
  let a1 = Foo(5, 6, "abc".to_string());
  let a2 = Foo(5, 6, "abc".to_string());
  let b = Foo(5, 7, "def".to_string());

  assert_eq!(a1, a1);
  assert_eq!(a2, a1);
  assert!(!(a1 == b));

  assert!(a1 != b);
  assert!(!(a1 != a1));
  assert!(!(a2 != a1));
}
