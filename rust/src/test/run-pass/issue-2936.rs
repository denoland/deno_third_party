// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


trait bar<T> {
    fn get_bar(&self) -> T;
}

fn foo<T, U: bar<T>>(b: U) -> T {
    b.get_bar()
}

struct cbar {
    x: isize,
}

impl bar<isize> for cbar {
    fn get_bar(&self) -> isize {
        self.x
    }
}

fn cbar(x: isize) -> cbar {
    cbar {
        x: x
    }
}

pub fn main() {
    let x: isize = foo::<isize, cbar>(cbar(5));
    assert_eq!(x, 5);
}
