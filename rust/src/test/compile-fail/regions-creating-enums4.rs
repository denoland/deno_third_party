// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

enum ast<'a> {
    num(usize),
    add(&'a ast<'a>, &'a ast<'a>)
}

fn mk_add_bad2<'a,'b>(x: &'a ast<'a>, y: &'a ast<'a>, z: &ast) -> ast<'b> {
    ast::add(x, y) //~ ERROR cannot infer
}

fn main() {
}
