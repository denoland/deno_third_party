// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-lldb

// compile-flags:-g
// gdb-command:run

// Test whether compiling a recursive enum definition crashes debug info generation. The test case
// is taken from issue #11083.

#![allow(unused_variables)]
#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

pub struct Window<'a> {
    callbacks: WindowCallbacks<'a>
}

struct WindowCallbacks<'a> {
    pos_callback: Option<Box<FnMut(&Window, i32, i32) + 'a>>,
}

fn main() {
    let x = WindowCallbacks { pos_callback: None };
}
