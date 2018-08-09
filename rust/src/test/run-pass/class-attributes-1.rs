// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pp-exact - Make sure we actually print the attributes
#![allow(unused_attribute)]
#![feature(custom_attribute)]

struct cat {
    name: String,
}

impl Drop for cat {
    #[cat_dropper]
    fn drop(&mut self) { println!("{} landed on hir feet" , self . name); }
}


#[cat_maker]
fn cat(name: String) -> cat { cat{name: name,} }

pub fn main() { let _kitty = cat("Spotty".to_string()); }
