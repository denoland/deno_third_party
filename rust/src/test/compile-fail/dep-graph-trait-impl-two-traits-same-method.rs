// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that adding an impl to a trait `Foo` DOES affect functions
// that only use `Bar` if they have methods in common.

// compile-flags: -Z query-dep-graph

#![feature(rustc_attrs)]
#![allow(dead_code)]
#![allow(unused_imports)]

fn main() { }

pub trait Foo: Sized {
    fn method(self) { }
}

pub trait Bar: Sized {
    fn method(self) { }
}

mod x {
    use {Foo, Bar};

    #[rustc_if_this_changed]
    impl Foo for u32 { }

    impl Bar for char { }
}

mod y {
    use {Foo, Bar};

    #[rustc_then_this_would_need(TypeckTables)] //~ ERROR OK
    pub fn with_char() {
        char::method('a');
    }
}

mod z {
    use y;

    #[rustc_then_this_would_need(TypeckTables)] //~ ERROR no path
    pub fn z() {
        y::with_char();
    }
}
