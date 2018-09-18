// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::thread;
use std::rc::Rc;

#[derive(Debug)]
struct Port<T>(Rc<T>);

fn main() {
    #[derive(Debug)]
    struct foo {
      _x: Port<()>,
    }

    impl Drop for foo {
        fn drop(&mut self) {}
    }

    fn foo(x: Port<()>) -> foo {
        foo {
            _x: x
        }
    }

    let x = foo(Port(Rc::new(())));

    thread::spawn(move|| {
        //~^ ERROR `std::rc::Rc<()>: std::marker::Send` is not satisfied
        let y = x;
        println!("{:?}", y);
    });
}
