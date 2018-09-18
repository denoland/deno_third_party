// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


mod a {
    pub trait Foo {
        fn foo() -> Self;
    }

    impl Foo for isize {
        fn foo() -> isize {
            3
        }
    }

    impl Foo for usize {
        fn foo() -> usize {
            5
        }
    }
}

pub fn main() {
    let x: isize = a::Foo::foo();
    let y: usize = a::Foo::foo();
    assert_eq!(x, 3);
    assert_eq!(y, 5);
}
