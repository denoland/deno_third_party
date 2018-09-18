// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub use sub_foo::Foo;
pub use self::Bar as Baz;
pub use sub_foo::Boz;
pub use sub_foo::Bort;

pub trait Bar {
    fn bar() -> Self;
}

impl Bar for isize {
    fn bar() -> isize { 84 }
}

pub mod sub_foo {
    pub trait Foo {
        fn foo() -> Self;
    }

    impl Foo for isize {
        fn foo() -> isize { 42 }
    }

    pub struct Boz {
        unused_str: String
    }

    impl Boz {
        pub fn boz(i: isize) -> bool {
            i > 0
        }
    }

    pub enum Bort {
        Bort1,
        Bort2
    }

    impl Bort {
        pub fn bort() -> String {
            "bort()".to_string()
        }
    }
}
