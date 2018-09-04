// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait Foo {
    fn dummy(&self) { }
}

struct A;

impl Foo for A {}

struct B<'a>(&'a (Foo+'a));

fn foo<'a>(a: &Foo) -> B<'a> {
    B(a)    //~ ERROR 22:5: 22:9: explicit lifetime required in the type of `a` [E0621]
}

fn main() {
    let _test = foo(&A);
}
