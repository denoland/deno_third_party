// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



pub fn f() -> isize { return 1; }

pub mod foo {
    pub fn f() -> isize { return 2; }
    pub fn g() {
        assert_eq!(f(), 2);
        assert_eq!(::f(), 1);
    }
}

pub fn main() { return foo::g(); }
