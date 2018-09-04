// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z parse-only

struct S {
 y: isize
}

impl Cmp, ToString for S {
//~^ ERROR: expected one of `!`, `(`, `+`, `::`, `<`, `for`, `where`, or `{`, found `,`
  fn eq(&&other: S) { false }
  fn to_string(&self) -> String { "hi".to_string() }
}
