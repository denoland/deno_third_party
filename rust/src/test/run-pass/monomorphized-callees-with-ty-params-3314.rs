// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

trait Serializer {
}

trait Serializable {
    fn serialize<S:Serializer>(&self, s: S);
}

impl Serializable for isize {
    fn serialize<S:Serializer>(&self, _s: S) { }
}

struct F<A> { a: A }

impl<A:Serializable> Serializable for F<A> {
    fn serialize<S:Serializer>(&self, s: S) {
        self.a.serialize(s);
    }
}

impl Serializer for isize {
}

pub fn main() {
    let foo = F { a: 1 };
    foo.serialize(1);

    let bar = F { a: F {a: 1 } };
    bar.serialize(2);
}
