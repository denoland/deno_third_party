// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Tests that unary structs can be mutably borrowed.

struct Empty;

trait T<U> {
    fn next(&mut self) -> Option<U>;
}
impl T<isize> for Empty {
    fn next(&mut self) -> Option<isize> { None }
}

fn do_something_with(a : &mut T<isize>) {
    println!("{:?}", a.next())
}

pub fn main() {
    do_something_with(&mut Empty);
}
