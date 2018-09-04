// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct X(isize);

enum Enum {
    Variant1,
    Variant2
}

impl Drop for X {
    fn drop(&mut self) {}
}
impl Drop for Enum {
    fn drop(&mut self) {}
}

fn main() {
    let foo = X(1);
    drop(foo);
    match foo { //~ ERROR use of moved value
        X(1) => (),
        _ => unreachable!()
    }

    let e = Enum::Variant2;
    drop(e);
    match e { //~ ERROR use of moved value
        Enum::Variant1 => unreachable!(),
        Enum::Variant2 => ()
    }
}
