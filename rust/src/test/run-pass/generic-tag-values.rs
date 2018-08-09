// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

enum noption<T> { some(T), }

struct Pair { x: isize, y: isize }

pub fn main() {
    let nop: noption<isize> = noption::some::<isize>(5);
    match nop { noption::some::<isize>(n) => { println!("{}", n); assert_eq!(n, 5); } }
    let nop2: noption<Pair> = noption::some(Pair{x: 17, y: 42});
    match nop2 {
      noption::some(t) => {
        println!("{}", t.x);
        println!("{}", t.y);
        assert_eq!(t.x, 17);
        assert_eq!(t.y, 42);
      }
    }
}
