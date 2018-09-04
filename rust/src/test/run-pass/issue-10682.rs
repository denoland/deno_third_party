// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Regression test for issue #10682
// Nested `proc` usage can't use outer owned data

// pretty-expanded FIXME #23616

#![allow(unknown_features)]
#![feature(box_syntax)]

fn work(_: Box<isize>) {}
fn foo<F:FnOnce()>(_: F) {}

pub fn main() {
  let a = box 1;
  foo(move|| { foo(move|| { work(a) }) })
}
