// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[derive(Copy, Clone)]
enum Q { R(Option<usize>) }

fn xyzzy(q: Q) -> usize {
    match q {
        Q::R(S) if S.is_some() => { 0 }
        _ => 1
    }
}


pub fn main() {
    assert_eq!(xyzzy(Q::R(Some(5))), 0);
}
