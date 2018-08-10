// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


fn mk() -> isize { return 1; }

fn chk(a: isize) { println!("{}", a); assert_eq!(a, 1); }

fn apply<T>(produce: fn() -> T,
            consume: fn(T)) {
    consume(produce());
}

pub fn main() {
    let produce: fn() -> isize = mk;
    let consume: fn(v: isize) = chk;
    apply::<isize>(produce, consume);
}
