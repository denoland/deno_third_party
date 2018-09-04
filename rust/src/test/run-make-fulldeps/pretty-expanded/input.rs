// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[crate_type="lib"]

// #13544

extern crate serialize;

#[derive(Encodable)] pub struct A;
#[derive(Encodable)] pub struct B(isize);
#[derive(Encodable)] pub struct C { x: isize }
#[derive(Encodable)] pub enum D {}
#[derive(Encodable)] pub enum E { y }
#[derive(Encodable)] pub enum F { z(isize) }
