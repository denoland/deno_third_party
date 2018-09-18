// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct Example {
    example: Box<Fn(i32) -> i32>
}

fn main() {
    let demo = Example {
        example: Box::new(|x| {
            x + 1
        })
    };

    demo.example(1);
    //~^ ERROR no method named `example`
    // (demo.example)(1);
}
