// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::mem;

enum ADT {
    First(u32, u32),
    Second(u64)
}

pub fn main() {
    assert!(mem::discriminant(&ADT::First(0,0)) == mem::discriminant(&ADT::First(1,1)));
    assert!(mem::discriminant(&ADT::Second(5))  == mem::discriminant(&ADT::Second(6)));
    assert!(mem::discriminant(&ADT::First(2,2)) != mem::discriminant(&ADT::Second(2)));

    let _ = mem::discriminant(&10);
    let _ = mem::discriminant(&"test");
}

