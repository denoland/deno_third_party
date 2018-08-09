// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that an impl with only one bound region `'a` cannot be used to
// satisfy a constraint where there are two bound regions.

trait Foo<X> {
    fn foo(&self, x: X) { }
}

fn want_foo2<T>()
    where T : for<'a,'b> Foo<(&'a isize, &'b isize)>
{
}

fn want_foo1<T>()
    where T : for<'z> Foo<(&'z isize, &'z isize)>
{
}

///////////////////////////////////////////////////////////////////////////
// Expressed as a where clause

struct SomeStruct;

impl<'a> Foo<(&'a isize, &'a isize)> for SomeStruct
{
}

fn a() { want_foo1::<SomeStruct>(); } // OK -- foo wants just one region
fn b() { want_foo2::<SomeStruct>(); } //~ ERROR E0277

fn main() { }
