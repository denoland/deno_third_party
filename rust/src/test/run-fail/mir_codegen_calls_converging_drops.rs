// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern:converging_fn called
// error-pattern:0 dropped
// error-pattern:exit

struct Droppable(u8);
impl Drop for Droppable {
    fn drop(&mut self) {
        eprintln!("{} dropped", self.0);
    }
}

fn converging_fn() {
    eprintln!("converging_fn called");
}

fn mir(d: Droppable) {
    converging_fn();
}

fn main() {
    let d = Droppable(0);
    mir(d);
    panic!("exit");
}
