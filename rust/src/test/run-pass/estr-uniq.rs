// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#![allow(dead_assignment)]

pub fn main() {
    let x : String = "hello".to_string();
    let _y : String = "there".to_string();
    let mut z = "thing".to_string();
    z = x;
    assert_eq!(z.as_bytes()[0], ('h' as u8));
    assert_eq!(z.as_bytes()[4], ('o' as u8));
}
