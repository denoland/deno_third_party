// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we can coerce an `@Object` to an `&Object`


trait Foo {
    fn foo(&self) -> usize;
    fn bar(&mut self) -> usize;
}

impl Foo for usize {
    fn foo(&self) -> usize {
        *self
    }

    fn bar(&mut self) -> usize {
        *self += 1;
        *self
    }
}

fn do_it_mut(obj: &mut Foo) {
    let x = obj.bar();
    let y = obj.foo();
    assert_eq!(x, y);

    do_it_imm(obj, y);
}

fn do_it_imm(obj: &Foo, v: usize) {
    let y = obj.foo();
    assert_eq!(v, y);
}

pub fn main() {
    let mut x: usize = 22;
    let obj = &mut x as &mut Foo;
    do_it_mut(obj);
    do_it_imm(obj, 23);
    do_it_mut(obj);
}
