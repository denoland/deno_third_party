// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

#![allow(unknown_features)]
#![feature(box_syntax)]

fn foo(x: Option<Box<isize>>, b: bool) -> isize {
    match x {
      None => { 1 }
      Some(ref x) if b => { *x.clone() }
      Some(_) => { 0 }
    }
}

pub fn main() {
    foo(Some(box 22), true);
    foo(Some(box 22), false);
    foo(None, true);
    foo(None, false);
}
