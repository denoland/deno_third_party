// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


struct sty(Vec<isize> );

fn unpack<F>(_unpack: F) where F: FnOnce(&sty) -> Vec<isize> {}

fn main() {
    let _foo = unpack(|s| {
        // Test that `s` is moved here.
        match *s { sty(v) => v } //~ ERROR cannot move out
    });
}
