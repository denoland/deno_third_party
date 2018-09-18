// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// A test where we (successfully) close over a reference into
// an object.


#![allow(unknown_features)]
#![feature(box_syntax)]

trait SomeTrait { fn get(&self) -> isize; }

impl<'a> SomeTrait for &'a isize {
    fn get(&self) -> isize {
        **self
    }
}

fn make_object<'a,A:SomeTrait+'a>(v: A) -> Box<SomeTrait+'a> {
    box v as Box<SomeTrait+'a>
}

fn main() {
    let i: isize = 22;
    let obj = make_object(&i);
    assert_eq!(22, obj.get());
}
