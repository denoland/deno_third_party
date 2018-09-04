// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(Clone, Debug)]
enum foo {
  a(usize),
  b(String),
}

fn check_log<T: std::fmt::Debug>(exp: String, v: T) {
    assert_eq!(exp, format!("{:?}", v));
}

pub fn main() {
    let mut x = Some(foo::a(22));
    let exp = "Some(a(22))".to_string();
    let act = format!("{:?}", x);
    assert_eq!(act, exp);
    check_log(exp, x);

    x = None;
    let exp = "None".to_string();
    let act = format!("{:?}", x);
    assert_eq!(act, exp);
    check_log(exp, x);
}
