// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use inner::Trait;

mod inner {
    pub struct Foo;
    pub trait Trait {
        fn f(&self);
        fn f_ufcs(&self);
    }

    impl Trait for Foo {
        fn f(&self) { }
        fn f_ufcs(&self) { }
    }
}

pub trait Outer {
    fn foo<T: Trait>(&self, t: T) { t.f(); }
    fn foo_ufcs<T: Trait>(&self, t: T) { T::f(&t); }
}

impl Outer for isize {}

pub fn foo<T: Outer>(t: T) {
    t.foo(inner::Foo);
}
pub fn foo_ufcs<T: Outer>(t: T) {
    T::foo_ufcs(&t, inner::Foo)
}
