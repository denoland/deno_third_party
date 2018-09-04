// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait IDummy {
    fn do_nothing(&self);
}

#[derive(Debug)]
struct A { a: isize }
#[derive(Debug)]
struct B<'a> { b: isize, pa: &'a A }

    impl IDummy for A {
        fn do_nothing(&self) {
            println!("A::do_nothing() is called");
        }
    }

impl<'a> B<'a> {
    fn get_pa(&self) -> &'a IDummy { self.pa as &'a IDummy }
}

pub fn main() {
    let sa = A { a: 100 };
    let sb = B { b: 200, pa: &sa };

    println!("sa is {:?}", sa);
    println!("sb is {:?}", sb);
}
