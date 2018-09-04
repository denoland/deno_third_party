// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// This is what the signature to spawn should look like with bare functions


fn spawn<T:Send>(val: T, f: fn(T)) {
    f(val);
}

fn f(i: isize) {
    assert_eq!(i, 100);
}

pub fn main() {
    spawn(100, f);
}
