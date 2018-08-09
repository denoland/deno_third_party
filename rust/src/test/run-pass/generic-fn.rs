// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_assignment)]

fn id<T>(x: T) -> T { return x; }

#[derive(Copy, Clone)]
struct Triple {x: isize, y: isize, z: isize}

pub fn main() {
    let mut x = 62;
    let mut y = 63;
    let a = 'a';
    let mut b = 'b';
    let p: Triple = Triple {x: 65, y: 66, z: 67};
    let mut q: Triple = Triple {x: 68, y: 69, z: 70};
    y = id::<isize>(x);
    println!("{}", y);
    assert_eq!(x, y);
    b = id::<char>(a);
    println!("{}", b);
    assert_eq!(a, b);
    q = id::<Triple>(p);
    x = p.z;
    y = q.z;
    println!("{}", y);
    assert_eq!(x, y);
}
