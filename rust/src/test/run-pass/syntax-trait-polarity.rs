// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

#![feature(optin_builtin_traits)]

struct TestType;

impl TestType {}

trait TestTrait {}

impl !Send for TestType {}

struct TestType2<T>(T);

impl<T> TestType2<T> {}

impl<T> !Send for TestType2<T> {}

fn main() {}
