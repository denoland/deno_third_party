// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// This is a regression test for the ICE from issue #10846.
//
// The original issue causing the ICE: the LUB-computations during
// type inference were encountering late-bound lifetimes, and
// asserting that such lifetimes should have already been substituted
// with a concrete lifetime.
//
// However, those encounters were occurring within the lexical scope
// of the binding for the late-bound lifetime; that is, the late-bound
// lifetimes were perfectly valid.  The core problem was that the type
// folding code was over-zealously passing back all lifetimes when
// doing region-folding, when really all clients of the region-folding
// case only want to see FREE lifetime variables, not bound ones.

// pretty-expanded FIXME #23616

#![allow(unknown_features)]
#![feature(box_syntax)]

pub fn main() {
    fn explicit() {
        fn test<F>(_x: Option<Box<F>>) where F: FnMut(Box<for<'a> FnMut(&'a isize)>) {}
        test(Some(box |_f: Box<for<'a> FnMut(&'a isize)>| {}));
    }

    // The code below is shorthand for the code above (and more likely
    // to represent what one encounters in practice).
    fn implicit() {
        fn test<F>(_x: Option<Box<F>>) where F: FnMut(Box<        FnMut(&   isize)>) {}
        test(Some(box |_f: Box<        FnMut(&   isize)>| {}));
    }

    explicit();
    implicit();
}
