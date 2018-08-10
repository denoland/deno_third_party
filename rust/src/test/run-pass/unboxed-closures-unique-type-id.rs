// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// This code used to produce the following ICE:
//
//    error: internal compiler error: get_unique_type_id_of_type() -
//    unexpected type: closure,
//    TyClosure(syntax::ast::DefId{krate: 0, node: 66},
//    ReScope(63))
//
// This is a regression test for issue #17021.
//
// compile-flags: -g

use std::ptr;

pub fn replace_map<'a, T, F>(src: &mut T, prod: F) where F: FnOnce(T) -> T {
    unsafe { *src = prod(ptr::read(src as *mut T as *const T)); }
}

pub fn main() {
    let mut a = 7;
    let b = &mut a;
    replace_map(b, |x: usize| x * 2);
    assert_eq!(*b, 14);
}
