// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// run-pass
// aux-build:lint_plugin_test.rs
// ignore-stage1
// compile-flags: -Z extra-plugins=lint_plugin_test

#![allow(dead_code)]

fn lintme() { } //~ WARNING item is named 'lintme'

#[allow(test_lint)]
pub fn main() {
    fn lintme() { }
}
