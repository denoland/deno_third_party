// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:default_type_params_xc.rs

// pretty-expanded FIXME #23616

extern crate default_type_params_xc;

struct Vec<T, A = default_type_params_xc::Heap>(Option<(T,A)>);

struct Foo;

fn main() {
    let _a = Vec::<isize>(None);
    let _b = Vec::<isize, default_type_params_xc::FakeHeap>(None);
    let _c = default_type_params_xc::FakeVec::<isize> { f: None };
    let _d = default_type_params_xc::FakeVec::<isize, Foo> { f: None };
}
