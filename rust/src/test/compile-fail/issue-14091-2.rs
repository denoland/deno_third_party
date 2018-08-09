// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern: cannot apply unary operator `!` to type `BytePos`

// Very

// sensitive
pub struct BytePos(pub u32);

// to particular

// line numberings / offsets

fn main() {
    let x = BytePos(1);

    assert!(x, x);
}
