// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct yes1<'a> {
  x: &'a usize,
}

struct yes2<'a> {
  x: &'a usize,
}

struct StructDecl {
    a: &'a isize, //~ ERROR use of undeclared lifetime name `'a`
    b: &'a isize, //~ ERROR use of undeclared lifetime name `'a`
}


fn main() {}
