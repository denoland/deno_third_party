// Copyright 2012-2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: --cfg foo

// pretty-expanded FIXME #23616

struct Foo {
    #[cfg(fail)]
    bar: baz,
    foo: isize,
}

struct Foo2 {
    #[cfg(foo)]
    foo: isize,
}

enum Bar1 {
    Bar1_1,
    #[cfg(fail)]
    Bar1_2(NotAType),
}

enum Bar2 {
    #[cfg(fail)]
    Bar2_1(NotAType),
}

enum Bar3 {
    Bar3_1 {
        #[cfg(fail)]
        foo: isize,
        bar: isize,
    }
}

pub fn main() {
    let _f = Foo { foo: 3 };
    let _f = Foo2 { foo: 3 };

    match Bar1::Bar1_1 {
        Bar1::Bar1_1 => {}
    }

    let _f = Bar3::Bar3_1 { bar: 3 };
}
