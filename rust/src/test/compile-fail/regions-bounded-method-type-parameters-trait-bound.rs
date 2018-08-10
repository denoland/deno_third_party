// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that explicit region bounds are allowed on the various
// nominal types (but not on other types) and that they are type
// checked.

struct Inv<'a> { // invariant w/r/t 'a
    x: &'a mut &'a isize
}

trait Foo<'x> {
    fn method<'y:'x>(self, y: Inv<'y>);
}

fn caller1<'a,'b,F:Foo<'a>>(a: Inv<'a>, b: Inv<'b>, f: F) {
    // Here the value provided for 'y is 'a, and hence 'a:'a holds.
    f.method(a);
}

fn caller2<'a,'b,F:Foo<'a>>(a: Inv<'a>, b: Inv<'b>, f: F) {
    // Here the value provided for 'y is 'b, and hence 'b:'a does not hold.
    f.method(b); //~ ERROR 30:7: 30:13: lifetime mismatch [E0623]
}

fn caller3<'a,'b:'a,F:Foo<'a>>(a: Inv<'a>, b: Inv<'b>, f: F) {
    // Here the value provided for 'y is 'b, and hence 'b:'a holds.
    f.method(b);
}

fn main() { }
