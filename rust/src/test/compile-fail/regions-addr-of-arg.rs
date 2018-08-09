// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that taking the address of an argument yields a lifetime
// bounded by the current function call.

fn foo(a: isize) {
    let _p: &'static isize = &a; //~ ERROR `a` does not live long enough
}

fn bar(a: isize) {
    let _q: &isize = &a;
}

fn zed<'a>(a: isize) -> &'a isize {
    &a //~ ERROR `a` does not live long enough
}

fn main() {
}
