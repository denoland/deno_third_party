// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-pretty issue #37195

#[path = "mod_dir_simple"]
mod biscuits {
    pub mod test;
}

#[path = "mod_dir_simple"]
mod gravy {
    pub mod test;
}

pub fn main() {
    assert_eq!(biscuits::test::foo(), 10);
    assert_eq!(gravy::test::foo(), 10);
}
