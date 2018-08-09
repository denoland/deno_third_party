// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

#![allow(unused_imports)]
#![feature(start, no_core, core)]
#![no_core]

extern crate std;
extern crate std as zed;

use std::str;
use zed::str as x;

use std::io::{self, Error as IoError, Result as IoResult};
use std::error::{self as foo};
mod baz {
    pub use std::str as x;
}

#[start]
pub fn start(_: isize, _: *const *const u8) -> isize { 0 }
