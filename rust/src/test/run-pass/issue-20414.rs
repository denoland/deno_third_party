// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

trait Trait {
        fn method(self) -> isize;
}

struct Wrapper<T> {
        field: T
}

impl<'a, T> Trait for &'a Wrapper<T> where &'a T: Trait {
    fn method(self) -> isize {
        let r: &'a T = &self.field;
        Trait::method(r); // these should both work
        r.method()
    }
}

fn main() {}
