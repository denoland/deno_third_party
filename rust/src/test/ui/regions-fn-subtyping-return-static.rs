// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// In this fn, the type `F` is a function that takes a reference to a
// struct and returns another reference with the same lifetime.
//
// Meanwhile, the bare fn `foo` takes a reference to a struct with
// *ANY* lifetime and returns a reference with the 'static lifetime.
// This can safely be considered to be an instance of `F` because all
// lifetimes are sublifetimes of 'static.

#![allow(dead_code)]
#![allow(unused_variables)]

struct S;

// Given 'cx, return 'cx
type F = for<'cx> fn(&'cx S) -> &'cx S;
fn want_F(f: F) { }

// Given anything, return 'static
type G = for<'cx> fn(&'cx S) -> &'static S;
fn want_G(f: G) { }

// Should meet both.
fn foo(x: &S) -> &'static S {
    panic!()
}

// Should meet both.
fn bar<'a,'b>(x: &'a S) -> &'b S {
    panic!()
}

// Meets F, but not G.
fn baz(x: &S) -> &S {
    panic!()
}

fn supply_F() {
    want_F(foo);

    // FIXME(#33684) -- this should be a subtype, but current alg. rejects it incorrectly
    want_F(bar); //~ ERROR E0308

    want_F(baz);
}

pub fn main() {
}
