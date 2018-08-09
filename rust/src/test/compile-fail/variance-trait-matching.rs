// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

// Get<T> is covariant in T
trait Get<T> {
    fn get(&self) -> T;
}

struct Cloner<T:Clone> {
    t: T
}

impl<T:Clone> Get<T> for Cloner<T> {
    fn get(&self) -> T {
        self.t.clone()
    }
}

fn get<'a, G>(get: &G) -> i32
    where G : Get<&'a i32>
{
    // This fails to type-check because, without variance, we can't
    // use `G : Get<&'a i32>` as evidence that `G : Get<&'b i32>`,
    // even if `'a : 'b`.
    pick(get, &22) //~ ERROR 34:5: 34:9: explicit lifetime required in the type of `get` [E0621]
}

fn pick<'b, G>(get: &'b G, if_odd: &'b i32) -> i32
    where G : Get<&'b i32>
{
    let v = *get.get();
    if v % 2 != 0 { v } else { *if_odd }
}

fn main() {
    let x = Cloner { t: &23 };
    let y = get(&x);
    assert_eq!(y, 23);
}
