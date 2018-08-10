// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// https://github.com/rust-lang/rust/issues/48821

#![feature(const_fn, const_let)]

const fn foo(i: usize) -> usize {
    let x = i;
    x
}

static FOO: usize = foo(42);

const fn bar(mut i: usize) -> usize {
    i += 8;
    let x = &i;
    *x
}

static BAR: usize = bar(42);

const fn boo(mut i: usize) -> usize {
    {
        let mut x = i;
        x += 10;
        i = x;
    }
    i
}

static BOO: usize = boo(42);

fn main() {
    assert!(FOO == 42);
    assert!(BAR == 50);
    assert!(BOO == 52);
}
