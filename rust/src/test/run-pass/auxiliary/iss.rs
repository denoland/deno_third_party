// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name="issue6919_3"]

// part of issue-6919.rs

pub struct C<K> where K: FnOnce() {
    pub k: K,
}

fn no_op() { }
pub const D : C<fn()> = C {
    k: no_op as fn()
};
