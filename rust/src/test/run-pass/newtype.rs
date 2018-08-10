// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(Copy, Clone)]
struct mytype(Mytype);

#[derive(Copy, Clone)]
struct Mytype {
    compute: fn(mytype) -> isize,
    val: isize,
}

fn compute(i: mytype) -> isize {
    let mytype(m) = i;
    return m.val + 20;
}

pub fn main() {
    let myval = mytype(Mytype{compute: compute, val: 30});
    println!("{}", compute(myval));
    let mytype(m) = myval;
    assert_eq!((m.compute)(myval), 50);
}
