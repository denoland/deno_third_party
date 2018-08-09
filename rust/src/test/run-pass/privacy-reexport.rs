// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// aux-build:privacy_reexport.rs

// pretty-expanded FIXME #23616

extern crate privacy_reexport;

pub fn main() {
    // Check that public extern crates are visible to outside crates
    privacy_reexport::core::cell::Cell::new(0);

    privacy_reexport::bar::frob();
}
