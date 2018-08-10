// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name="socketlib"]
#![crate_type = "lib"]

pub mod socket {
    pub struct socket_handle {
        sockfd: u32,
    }

    impl Drop for socket_handle {
        fn drop(&mut self) {
            /* c::close(self.sockfd); */
        }
    }

    pub fn socket_handle(x: u32) -> socket_handle {
        socket_handle {
            sockfd: x
        }
    }
}
