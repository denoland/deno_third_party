// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that we successfully handle methods where the `self` type has
// an early-bound lifetime. Issue #18208.

// pretty-expanded FIXME #23616

#![allow(dead_code)]

use std::marker;

struct Cursor<'a> {
    m: marker::PhantomData<&'a ()>
}

trait CursorNavigator {
    fn init_cursor<'a, 'b:'a>(&'a self, cursor: &mut Cursor<'b>) -> bool;
}

struct SimpleNavigator;

impl CursorNavigator for SimpleNavigator {
    fn init_cursor<'a, 'b: 'a>(&'a self, _cursor: &mut Cursor<'b>) -> bool {
        false
    }
}

fn main() {
    let mut c = Cursor { m: marker::PhantomData };
    let n = SimpleNavigator;
    n.init_cursor(&mut c);
}
