// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

trait Next {
    type Next: Next;
}

struct GetNext<T: Next> { t: T }

impl<T: Next> Next for GetNext<T> {
    //~^ ERROR overflow evaluating the requirement
    type Next = <GetNext<T> as Next>::Next;
}

fn main() {}
