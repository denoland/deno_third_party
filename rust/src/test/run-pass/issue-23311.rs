// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we do not ICE when pattern matching an array against a slice.

#![feature(slice_patterns)]

fn main() {
    match "foo".as_bytes() {
        b"food" => (),
        &[b'f', ..] => (),
        _ => ()
    }
}
