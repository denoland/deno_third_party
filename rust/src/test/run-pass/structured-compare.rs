// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



#[derive(Copy, Clone, Debug)]
enum foo { large, small, }

impl PartialEq for foo {
    fn eq(&self, other: &foo) -> bool {
        ((*self) as usize) == ((*other) as usize)
    }
    fn ne(&self, other: &foo) -> bool { !(*self).eq(other) }
}

pub fn main() {
    let a = (1, 2, 3);
    let b = (1, 2, 3);
    assert_eq!(a, b);
    assert!((a != (1, 2, 4)));
    assert!((a < (1, 2, 4)));
    assert!((a <= (1, 2, 4)));
    assert!(((1, 2, 4) > a));
    assert!(((1, 2, 4) >= a));
    let x = foo::large;
    let y = foo::small;
    assert!((x != y));
    assert_eq!(x, foo::large);
    assert!((x != foo::small));
}
