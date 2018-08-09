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

struct socket {
    sock: isize,

}

impl Drop for socket {
    fn drop(&mut self) {}
}

impl socket {
    pub fn set_identity(&self)  {
        closure(|| setsockopt_bytes(self.sock.clone()))
    }
}

fn socket() -> socket {
    socket {
        sock: 1
    }
}

fn closure<F>(f: F) where F: FnOnce() { f() }

fn setsockopt_bytes(_sock: isize) { }

pub fn main() {}
