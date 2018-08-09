// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-windows
// ignore-emscripten no threads support
// exec-env:RUST_LOG=debug

use std::cell::Cell;
use std::fmt;
use std::thread;

struct Foo(Cell<isize>);

impl fmt::Debug for Foo {
    fn fmt(&self, _fmt: &mut fmt::Formatter) -> fmt::Result {
        let Foo(ref f) = *self;
        assert_eq!(f.get(), 0);
        f.set(1);
        Ok(())
    }
}

pub fn main() {
    thread::spawn(move|| {
        let mut f = Foo(Cell::new(0));
        println!("{:?}", f);
        let Foo(ref mut f) = f;
        assert_eq!(f.get(), 1);
    }).join().ok().unwrap();
}
