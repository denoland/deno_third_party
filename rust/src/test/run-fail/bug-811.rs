// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern:quux

use std::marker::PhantomData;

fn test00_start(ch: chan_t<isize>, message: isize) {
    send(ch, message);
}

type task_id = isize;
type port_id = isize;

struct chan_t<T> {
    task: task_id,
    port: port_id,
    marker: PhantomData<*mut T>,
}

fn send<T: Send>(_ch: chan_t<T>, _data: T) {
    panic!();
}

fn main() {
    panic!("quux");
}
