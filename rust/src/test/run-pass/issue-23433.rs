// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Don't fail if we encounter a NonNull<T> where T is an unsized type

use std::ptr::NonNull;

fn main() {
    let mut a = [0u8; 5];
    let b: Option<NonNull<[u8]>> = Some(NonNull::from(&mut a));
    match b {
        Some(_) => println!("Got `Some`"),
        None => panic!("Unexpected `None`"),
    }
}
