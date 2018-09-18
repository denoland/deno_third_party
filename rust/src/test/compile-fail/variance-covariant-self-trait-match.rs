// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

// Test that even when `Self` is only used in covariant position, it
// is treated as invariant.

trait Get {
    fn get() -> Self;
}

fn get_min_from_max<'min, 'max, G>()
    where 'max : 'min, G : 'max, &'max G : Get
{
    // Previously OK, now an error as traits are invariant.
    impls_get::<&'min G>(); //~ ERROR mismatched types
}

fn get_max_from_min<'min, 'max, G>()
    where 'max : 'min, G : 'max, &'min G : Get
{
    impls_get::<&'max G>(); //~ ERROR mismatched types
}

fn impls_get<G>() where G : Get { }

fn main() { }
