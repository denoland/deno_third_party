// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::fmt::Debug;

trait MyTrait<T> {
    fn get(&self) -> T;
}

#[derive(Copy, Clone)]
struct MyType {
    dummy: usize
}

impl MyTrait<usize> for MyType {
    fn get(&self) -> usize { self.dummy }
}

impl MyTrait<u8> for MyType {
    fn get(&self) -> u8 { self.dummy as u8 }
}

fn test_eq<T,M>(m: M, v: T)
where T : Eq + Debug,
      M : MyTrait<T>
{
    assert_eq!(m.get(), v);
}

pub fn main() {
    let value = MyType { dummy: 256 + 22 };
    test_eq::<usize, _>(value, value.dummy);
    test_eq::<u8, _>(value, value.dummy as u8);
}
