// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(missing_docs)]
//! this tests the syntax of `thread_local!`

mod foo {
    mod bar {
        thread_local! {
            // no docs
            #[allow(unused)]
            static FOO: i32 = 42;
            /// docs
            pub static BAR: String = String::from("bar");

            // look at these restrictions!!
            pub(crate) static BAZ: usize = 0;
            pub(in foo) static QUUX: usize = 0;
        }
        thread_local!(static SPLOK: u32 = 0);
    }
}

fn main() {}
