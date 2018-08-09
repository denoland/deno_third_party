// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -Z no-landing-pads -C codegen-units=1
// error-pattern:converging_fn called
// ignore-cloudabi no std::process

use std::io::{self, Write};

struct Droppable;
impl Drop for Droppable {
    fn drop(&mut self) {
        ::std::process::exit(1)
    }
}

fn converging_fn() {
    panic!("converging_fn called")
}

fn mir(d: Droppable) {
    let x = Droppable;
    converging_fn();
    drop(x);
    drop(d);
}

fn main() {
    mir(Droppable);
}
