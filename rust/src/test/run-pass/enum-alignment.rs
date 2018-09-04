// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::mem;

fn addr_of<T>(ptr: &T) -> usize {
    ptr as *const T as usize
}

fn is_aligned<T>(ptr: &T) -> bool {
    unsafe {
        let addr: usize = mem::transmute(ptr);
        (addr % mem::min_align_of::<T>()) == 0
    }
}

pub fn main() {
    let x = Some(0u64);
    match x {
        None => panic!(),
        Some(ref y) => assert!(is_aligned(y))
    }
}
