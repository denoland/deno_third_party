// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

#![allow(warnings)]

struct S<T>(T);

static s1: S<S<usize>>=S(S(0));
static s2: S<usize>=S(0);

fn main() {
    let foo: S<S<usize>>=S(S(0));
    let foo: S<usize>=S(0);
}
