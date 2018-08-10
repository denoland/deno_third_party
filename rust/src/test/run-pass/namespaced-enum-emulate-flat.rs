// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

pub use Foo::*;
use nest::{Bar, D, E, F};

pub enum Foo {
    A,
    B(isize),
    C { a: isize },
}

impl Foo {
    pub fn foo() {}
}

fn _f(f: Foo) {
    match f {
        A | B(_) | C { .. } => {}
    }
}

mod nest {
    pub use self::Bar::*;

    pub enum Bar {
        D,
        E(isize),
        F { a: isize },
    }

    impl Bar {
        pub fn foo() {}
    }
}

fn _f2(f: Bar) {
    match f {
        D | E(_) | F { .. } => {}
    }
}

fn main() {}
