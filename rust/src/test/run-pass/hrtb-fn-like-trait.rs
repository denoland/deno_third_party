// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// A basic test of using a higher-ranked trait bound.


trait FnLike<A,R> {
    fn call(&self, arg: A) -> R;
}

struct Identity;

impl<'a, T> FnLike<&'a T, &'a T> for Identity {
    fn call(&self, arg: &'a T) -> &'a T {
        arg
    }
}

fn call_repeatedly<F>(f: F)
    where F : for<'a> FnLike<&'a isize, &'a isize>
{
    let x = 3;
    let y = f.call(&x);
    assert_eq!(3, *y);
}

fn main() {
    call_repeatedly(Identity);
}
