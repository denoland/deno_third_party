// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that we do not permit moves from &[] matched by a vec pattern.

#![feature(slice_patterns)]

#[derive(Clone, Debug)]
struct Foo {
    string: String
}

pub fn main() {
    let x = vec![
        Foo { string: "foo".to_string() },
        Foo { string: "bar".to_string() },
        Foo { string: "baz".to_string() }
    ];
    let x: &[Foo] = &x;
    match *x {
        [_, ref tail..] => {
            match tail {
                &[Foo { string: a },
                //~^ ERROR cannot move out of type `[Foo]`
                //~| cannot move out
                //~| to prevent move
                  Foo { string: b }] => {
                }
                _ => {
                    unreachable!();
                }
            }
            let z = tail[0].clone();
            println!("{:?}", z);
        }
        _ => {
            unreachable!();
        }
    }
}
