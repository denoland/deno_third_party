// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


fn ho<F>(f: F) -> isize where F: FnOnce(isize) -> isize { let n: isize = f(3); return n; }

fn direct(x: isize) -> isize { return x + 1; }

pub fn main() {
    let a: isize = direct(3); // direct
    let b: isize = ho(direct); // indirect unbound

    assert_eq!(a, b);
}
