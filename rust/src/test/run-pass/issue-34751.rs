// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// #34751 ICE: 'rustc' panicked at 'assertion failed: !substs.has_regions_escaping_depth(0)'

#[allow(dead_code)]

use std::marker::PhantomData;

fn f<'a>(PhantomData::<&'a u8>: PhantomData<&'a u8>) {}

fn main() {}
