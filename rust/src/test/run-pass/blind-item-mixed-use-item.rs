// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

mod m {
    pub fn f<T>(_: T, _: ()) { }
    pub fn g<T>(_: T, _: ()) { }
}

const BAR: () = ();
struct Data;
use m::f;

fn main() {
    const BAR2: () = ();
    struct Data2;
    use m::g;

    f(Data, BAR);
    g(Data2, BAR2);
}
