// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


fn each<T, F>(x: &[T], mut f: F) where F: FnMut(&T) {
    for val in x {
        f(val)
    }
}

fn main() {
    let mut sum = 0_usize;
    let elems = [ 1_usize, 2, 3, 4, 5 ];
    each(&elems, |val| sum += *val);
    assert_eq!(sum, 15);
}
