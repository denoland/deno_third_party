// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

fn foo<'a>(a: &'a isize) { }
fn bar(a: &'static isize) { }
fn baz<'let>(a: &'let isize) { } //~ ERROR lifetimes cannot use keyword names
//~^ ERROR lifetimes cannot use keyword names
fn zab<'self>(a: &'self isize) { } //~ ERROR lifetimes cannot use keyword names
//~^ ERROR lifetimes cannot use keyword names
fn main() { }
