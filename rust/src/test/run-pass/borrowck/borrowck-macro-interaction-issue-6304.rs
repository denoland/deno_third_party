// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that we do not ICE when compiling this
// macro, which reuses the expression `$id`


#![feature(box_patterns)]
#![feature(box_syntax)]

struct Foo {
  a: isize
}

pub enum Bar {
  Bar1, Bar2(isize, Box<Bar>),
}

impl Foo {
  fn elaborate_stm(&mut self, s: Box<Bar>) -> Box<Bar> {
    macro_rules! declare {
      ($id:expr, $rest:expr) => ({
        self.check_id($id);
        box Bar::Bar2($id, $rest)
      })
    }
    match s {
      box Bar::Bar2(id, rest) => declare!(id, self.elaborate_stm(rest)),
      _ => panic!()
    }
  }

  fn check_id(&mut self, s: isize) { panic!() }
}

pub fn main() { }
