// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;

fn check_strs(actual: &str, expected: &str) -> bool {
    if actual != expected {
        println!("Found {}, but expected {}", actual, expected);
        return false;
    }
    return true;
}

pub fn main() {
    let mut table = HashMap::new();
    table.insert("one".to_string(), 1);
    table.insert("two".to_string(), 2);
    assert!(check_strs(&format!("{:?}", table), "{\"one\": 1, \"two\": 2}") ||
            check_strs(&format!("{:?}", table), "{\"two\": 2, \"one\": 1}"));
}
