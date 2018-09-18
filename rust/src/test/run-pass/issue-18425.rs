// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that codegen doesn't ICE when codegenning an array repeat
// expression with a count of 1 and a non-Copy element type.

// pretty-expanded FIXME #23616

fn main() {
    let _ = [Box::new(1_usize); 1];
}
