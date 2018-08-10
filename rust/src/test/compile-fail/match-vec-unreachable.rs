// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(slice_patterns)]
#![deny(unreachable_patterns)]

fn main() {
    let x: Vec<(isize, isize)> = Vec::new();
    let x: &[(isize, isize)] = &x;
    match *x {
        [a, (2, 3), _] => (),
        [(1, 2), (2, 3), b] => (), //~ ERROR unreachable pattern
        _ => ()
    }

    let x: Vec<String> = vec!["foo".to_string(),
                              "bar".to_string(),
                              "baz".to_string()];
    let x: &[String] = &x;
    match *x {
        [ref a, _, _, ..] => { println!("{}", a); }
        [_, _, _, _, _] => { } //~ ERROR unreachable pattern
        _ => { }
    }

    let x: Vec<char> = vec!['a', 'b', 'c'];
    let x: &[char] = &x;
    match *x {
        ['a', 'b', 'c', ref _tail..] => {}
        ['a', 'b', 'c'] => {} //~ ERROR unreachable pattern
        _ => {}
    }
}
