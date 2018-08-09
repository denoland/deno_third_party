// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z borrowck=compare

fn main() {
    let mut x = Box::new(0);
    let _u = x; // error shouldn't note this move
    x = Box::new(1);
    drop(x);
    let _ = (1,x); //~ ERROR use of moved value: `x` (Ast)
    //~^ ERROR use of moved value: `x` (Mir)
}
