// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(rustc_private)]

extern crate rustc_metadata;

use rustc_metadata::dynamic_lib::DynamicLibrary;
use std::path::Path;

pub fn main() {
    unsafe {
        let path = Path::new("libdylib.so");
        let a = DynamicLibrary::open(Some(&path)).unwrap();
        assert!(a.symbol::<isize>("fun1").is_ok());
        assert!(a.symbol::<isize>("fun2").is_err());
        assert!(a.symbol::<isize>("fun3").is_err());
        assert!(a.symbol::<isize>("fun4").is_ok());
        assert!(a.symbol::<isize>("fun5").is_ok());
    }
}
