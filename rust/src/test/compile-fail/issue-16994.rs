// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(rustc_attrs)]

fn cb<'a,T>(_x: Box<Fn((&'a i32, &'a (Vec<&'static i32>, bool))) -> T>) -> T {
    panic!()
}

#[rustc_error]
fn main() { //~ ERROR compilation successful
    cb(Box::new(|(k, &(ref v, b))| (*k, v.clone(), b)));
}
