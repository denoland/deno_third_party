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

// gdb-command:info locals
// gdb-check:No locals.
// gdb-command:continue

// gdb-command:info locals
// gdb-check:abc = 10
// gdb-command:continue

#![allow(unused_variables)]
#![feature(no_debug)]
#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

#[inline(never)]
fn id<T>(x: T) -> T {x}

fn function_with_debuginfo() {
    let abc = 10_usize;
    id(abc); // #break
}

#[no_debug]
fn function_without_debuginfo() {
    let abc = -57i32;
    id(abc); // #break
}

fn main() {
    function_without_debuginfo();
    function_with_debuginfo();
}
