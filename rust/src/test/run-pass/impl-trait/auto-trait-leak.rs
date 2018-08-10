// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Fast path, main can see the concrete type returned.
fn before() -> impl FnMut(i32) {
    let mut p = Box::new(0);
    move |x| *p = x
}

fn send<T: Send>(_: T) {}

fn main() {
    send(before());
    send(after());
}

// Deferred path, main has to wait until typeck finishes,
// to check if the return type of after is Send.
fn after() -> impl FnMut(i32) {
    let mut p = Box::new(0);
    move |x| *p = x
}
