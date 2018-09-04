// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::ops::*;
use test::Bencher;

// Overhead of dtors

struct HasDtor {
    _x: isize
}

impl Drop for HasDtor {
    fn drop(&mut self) {
    }
}

#[bench]
fn alloc_obj_with_dtor(b: &mut Bencher) {
    b.iter(|| {
        HasDtor { _x : 10 };
    })
}
