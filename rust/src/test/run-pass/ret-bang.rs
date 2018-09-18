// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.




fn my_err(s: String) -> ! { println!("{}", s); panic!(); }

fn okay(i: usize) -> isize {
    if i == 3 {
        my_err("I don't like three".to_string());
    } else {
        return 42;
    }
}

pub fn main() { okay(4); }
