// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

struct cat {
  done : extern fn(usize),
  meows : usize,
}

impl Drop for cat {
    fn drop(&mut self) {
        (self.done)(self.meows);
    }
}

fn cat(done: extern fn(usize)) -> cat {
    cat {
        meows: 0,
        done: done
    }
}

pub fn main() {}
