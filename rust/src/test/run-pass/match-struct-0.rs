// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


struct Foo{
    f : isize,
}

pub fn main() {
    let f = Foo{f: 1};
    match f {
        Foo{f: 0} => panic!(),
        Foo{..} => (),
    }
    match f {
        Foo{f: 0} => panic!(),
        Foo{f: _f} => (),
    }
    match f {
        Foo{f: 0} => panic!(),
        _ => (),
    }
}
