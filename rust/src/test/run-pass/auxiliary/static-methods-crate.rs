// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name="static_methods_crate"]
#![crate_type = "lib"]

pub trait read: Sized {
    fn readMaybe(s: String) -> Option<Self>;
}

impl read for isize {
    fn readMaybe(s: String) -> Option<isize> {
        s.parse().ok()
    }
}

impl read for bool {
    fn readMaybe(s: String) -> Option<bool> {
        match &*s {
          "true" => Some(true),
          "false" => Some(false),
          _ => None
        }
    }
}

pub fn read<T:read>(s: String) -> T {
    match read::readMaybe(s) {
      Some(x) => x,
      _ => panic!("read panicked!")
    }
}
