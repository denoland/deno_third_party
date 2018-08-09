// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct Foo {
    x: &bool,
    //~^ ERROR E0106
}
enum Bar {
    A(u8),
    B(&bool),
   //~^ ERROR E0106
}
type MyStr = &str;
        //~^ ERROR E0106

struct Baz<'a>(&'a str);
struct Buzz<'a, 'b>(&'a str, &'b str);

struct Quux {
    baz: Baz,
    //~^ ERROR E0106
    //~| expected lifetime parameter
    buzz: Buzz,
    //~^ ERROR E0106
    //~| expected 2 lifetime parameters
}

fn main() {
}
