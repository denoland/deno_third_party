// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check projection of an associated type out of a higher-ranked trait-bound
// in the context of a method definition in a trait.

pub trait Foo<T> {
    type A;

    fn get(&self, t: T) -> Self::A;
}

trait SomeTrait<I : for<'x> Foo<&'x isize>> {
    fn some_method(&self, arg: I::A);
    //~^ ERROR cannot extract an associated type from a higher-ranked trait bound in this context
}

trait AnotherTrait<I : for<'x> Foo<&'x isize>> {
    fn some_method(&self, arg: <I as Foo<&isize>>::A);
}

trait YetAnotherTrait<I : for<'x> Foo<&'x isize>> {
    fn some_method<'a>(&self, arg: <I as Foo<&'a isize>>::A);
}

pub fn main() {}
