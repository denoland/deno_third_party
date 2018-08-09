// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#![allow(unknown_features)]
#![feature(box_syntax)]

struct DroppableStruct;
enum DroppableEnum {
    DroppableVariant1, DroppableVariant2
}

static mut DROPPED: bool = false;

impl Drop for DroppableStruct {
    fn drop(&mut self) {
        unsafe { DROPPED = true; }
    }
}
impl Drop for DroppableEnum {
    fn drop(&mut self) {
        unsafe { DROPPED = true; }
    }
}

trait MyTrait { fn dummy(&self) { } }
impl MyTrait for Box<DroppableStruct> {}
impl MyTrait for Box<DroppableEnum> {}

struct Whatever { w: Box<MyTrait+'static> }
impl  Whatever {
    fn new(w: Box<MyTrait+'static>) -> Whatever {
        Whatever { w: w }
    }
}

fn main() {
    {
        let f: Box<_> = box DroppableStruct;
        let _a = Whatever::new(box f as Box<MyTrait>);
    }
    assert!(unsafe { DROPPED });
    unsafe { DROPPED = false; }
    {
        let f: Box<_> = box DroppableEnum::DroppableVariant1;
        let _a = Whatever::new(box f as Box<MyTrait>);
    }
    assert!(unsafe { DROPPED });
}
