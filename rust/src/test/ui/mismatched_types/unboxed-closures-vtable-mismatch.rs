// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(unboxed_closures)]

use std::ops::FnMut;

fn to_fn_mut<A,F:FnMut<A>>(f: F) -> F { f }

fn call_it<F:FnMut(isize,isize)->isize>(y: isize, mut f: F) -> isize {
//~^ NOTE required by `call_it`
    f(2, y)
}

pub fn main() {
    let f = to_fn_mut(|x: usize, y: isize| -> isize { (x as isize) + y });
    //~^ NOTE found signature of `fn(usize, isize) -> _`
    let z = call_it(3, f);
    //~^ ERROR type mismatch
    //~| NOTE expected signature of `fn(isize, isize) -> _`
    println!("{}", z);
}
