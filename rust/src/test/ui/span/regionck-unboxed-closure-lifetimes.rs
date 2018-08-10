// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
#![feature(rustc_attrs)]
use std::ops::FnMut;

fn main() { #![rustc_error] // rust-lang/rust#49855
    let mut f;
    {
        let c = 1;
        let c_ref = &c;
        //~^ ERROR `c` does not live long enough
        f = move |a: isize, b: isize| { a + b + *c_ref };
    }
    f.use_mut();
}

trait Fake { fn use_mut(&mut self) { } fn use_ref(&self) { }  }
impl<T> Fake for T { }
