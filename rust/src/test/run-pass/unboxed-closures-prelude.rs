// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Tests that the re-exports of `FnOnce` et al from the prelude work.

// pretty-expanded FIXME #23616

fn main() {
    let task: Box<Fn(isize) -> isize> = Box::new(|x| x);
    task(0);

    let mut task: Box<FnMut(isize) -> isize> = Box::new(|x| x);
    task(0);

    call(|x| x, 22);
}

fn call<F:FnOnce(isize) -> isize>(f: F, x: isize) -> isize {
    f(x)
}
