// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that the 'static bound on a proc influences lifetimes of
// region variables contained within (otherwise, region inference will
// give `x` a very short lifetime).

// pretty-expanded FIXME #23616

static i: usize = 3;
fn foo<F:FnOnce()+'static>(_: F) {}
fn read(_: usize) { }
pub fn main() {
    let x = &i;
    foo(move|| {
        read(*x);
    });
}
