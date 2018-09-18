// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// min-lldb-version: 310

// compile-flags:-g
// gdb-command:run
// lldb-command:run

#![allow(unused_variables)]
#![feature(box_syntax)]
#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

trait Trait {
    fn method(&self) -> isize { 0 }
}

struct Struct {
    a: isize,
    b: f64
}

impl Trait for Struct {}

// There is no real test here yet. Just make sure that it compiles without crashing.
fn main() {
    let stack_struct = Struct { a:0, b: 1.0 };
    let reference: &Trait = &stack_struct as &Trait;
    let unique: Box<Trait> = box Struct { a:2, b: 3.0 } as Box<Trait>;
}
