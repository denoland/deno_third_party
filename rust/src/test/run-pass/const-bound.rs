// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Make sure const bounds work on things, and test that a few types
// are const.

// pretty-expanded FIXME #23616

fn foo<T: Sync>(x: T) -> T { x }

struct F { field: isize }

pub fn main() {
    /*foo(1);
    foo("hi".to_string());
    foo(vec![1, 2, 3]);
    foo(F{field: 42});
    foo((1, 2));
    foo(@1);*/
    foo(Box::new(1));
}
