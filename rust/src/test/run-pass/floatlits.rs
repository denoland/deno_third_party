// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.




pub fn main() {
    let f = 4.999999999999f64;
    assert!((f > 4.90f64));
    assert!((f < 5.0f64));
    let g = 4.90000000001e-10f64;
    assert!((g > 5e-11f64));
    assert!((g < 5e-9f64));
}
