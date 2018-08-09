// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

fn iter_vec<T, F>(v: Vec<T>, mut f: F) where F: FnMut(&T) { for x in &v { f(x); } }

pub fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let mut sum = 0;
    iter_vec(v.clone(), |i| {
        iter_vec(v.clone(), |j| {
            sum += *i * *j;
        });
    });
    println!("{}", sum);
    assert_eq!(sum, 225);
}
