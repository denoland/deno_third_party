// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: --cfg fooA --cfg fooB

// fooA AND !bar

#[cfg(all(fooA, not(bar)))]
fn foo1() -> isize { 1 }

// !fooA AND !bar
#[cfg(all(not(fooA), not(bar)))]
fn foo2() -> isize { 2 }

// fooC OR (fooB AND !bar)
#[cfg(any(fooC, all(fooB, not(bar))))]
fn foo2() -> isize { 3 }

// fooA AND bar
#[cfg(all(fooA, bar))]
fn foo3() -> isize { 2 }

// !(fooA AND bar)
#[cfg(not(all(fooA, bar)))]
fn foo3() -> isize { 3 }

pub fn main() {
    assert_eq!(1, foo1());
    assert_eq!(3, foo2());
    assert_eq!(3, foo3());
}
