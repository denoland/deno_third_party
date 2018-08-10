// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


pub fn main() {
    match &[(Box::new(5),Box::new(7))] {
        ps => {
           let (ref y, _) = ps[0];
           assert_eq!(**y, 5);
        }
    }

    match Some(&[(Box::new(5),)]) {
        Some(ps) => {
           let (ref y,) = ps[0];
           assert_eq!(**y, 5);
        }
        None => ()
    }

    match Some(&[(Box::new(5),Box::new(7))]) {
        Some(ps) => {
           let (ref y, ref z) = ps[0];
           assert_eq!(**y, 5);
           assert_eq!(**z, 7);
        }
        None => ()
    }
}
