// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unknown_features)]
#![feature(box_syntax)]

//type t = { a: isize };
// type t = { a: bool };
type t = bool;

trait it {
    fn f(&self);
}

impl it for t {
    fn f(&self) { }
}

pub fn main() {
  //    let x = ({a: 4} as it);
  //   let y = box ({a: 4});
  //    let z = box ({a: 4} as it);
  //    let z = box ({a: true} as it);
    let z: Box<_> = box (box true as Box<it>);
    //  x.f();
    // y.f();
    // (*z).f();
    println!("ok so far...");
    z.f(); //segfault
}
