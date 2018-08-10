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

trait Bound {
    fn dummy(&self) { }
}

trait Trait {
    fn a<T>(&self, _: T) where T: Bound;
    fn b<T>(&self, _: T) where T: Bound;
    fn c<T: Bound>(&self, _: T);
    fn d<T: Bound>(&self, _: T);
}

impl Trait for bool {
    fn a<T: Bound>(&self, _: T) {}
    //^~ This gets rejected but should be accepted
    fn b<T>(&self, _: T) where T: Bound {}
    fn c<T: Bound>(&self, _: T) {}
    fn d<T>(&self, _: T) where T: Bound {}
}

fn main() {}
