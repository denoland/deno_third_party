// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Any copyright is dedicated to the Public Domain.
// http://creativecommons.org/publicdomain/zero/1.0/

// pp-exact

fn call_it(f: Box<FnMut(String) -> String>) { }

fn call_this<F>(f: F) where F: Fn(&str) + Send { }

fn call_that<F>(f: F) where F: for<'a> Fn(&'a isize, &'a isize) -> isize { }

fn call_extern(f: fn() -> isize) { }

fn call_abid_extern(f: extern "C" fn() -> isize) { }

pub fn main() { }
