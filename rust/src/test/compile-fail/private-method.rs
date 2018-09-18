// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern:method `nap` is private

mod kitties {
    pub struct cat {
        meows : usize,

        how_hungry : isize,
    }

    impl cat {
        fn nap(&self) {}
    }

    pub fn cat(in_x : usize, in_y : isize) -> cat {
        cat {
            meows: in_x,
            how_hungry: in_y
        }
    }
}

fn main() {
  let nyan : kitties::cat = kitties::cat(52, 99);
  nyan.nap();
}
