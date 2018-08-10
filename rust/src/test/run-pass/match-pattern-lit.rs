// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



fn altlit(f: isize) -> isize {
    match f {
      10 => { println!("case 10"); return 20; }
      11 => { println!("case 11"); return 22; }
      _  => panic!("the impossible happened")
    }
}

pub fn main() {
    assert_eq!(altlit(10), 20);
    assert_eq!(altlit(11), 22);
}
