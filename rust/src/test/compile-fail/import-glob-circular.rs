// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod circ1 {
    pub use circ2::f2;
    pub fn f1() { println!("f1"); }
    pub fn common() -> usize { return 0; }
}

mod circ2 {
    pub use circ1::f1;
    pub fn f2() { println!("f2"); }
    pub fn common() -> usize { return 1; }
}

mod test {
    use circ1::*;

    fn test() { f1066(); } //~ ERROR cannot find function `f1066` in this scope
}
