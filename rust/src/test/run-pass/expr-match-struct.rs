// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.





// Tests for match as expressions resulting in struct types
#[derive(Copy, Clone)]
struct R { i: isize }

fn test_rec() {
    let rs = match true { true => R {i: 100}, _ => panic!() };
    assert_eq!(rs.i, 100);
}

#[derive(Copy, Clone, Debug)]
enum mood { happy, sad, }

impl PartialEq for mood {
    fn eq(&self, other: &mood) -> bool {
        ((*self) as usize) == ((*other) as usize)
    }
    fn ne(&self, other: &mood) -> bool { !(*self).eq(other) }
}

fn test_tag() {
    let rs = match true { true => { mood::happy } false => { mood::sad } };
    assert_eq!(rs, mood::happy);
}

pub fn main() { test_rec(); test_tag(); }
