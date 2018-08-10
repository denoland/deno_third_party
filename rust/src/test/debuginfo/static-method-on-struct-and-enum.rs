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

// === GDB TESTS ===================================================================================

// gdb-command:run

// STRUCT
// gdb-command:print arg1
// gdb-check:$1 = 1
// gdb-command:print arg2
// gdb-check:$2 = 2
// gdb-command:continue

// ENUM
// gdb-command:print arg1
// gdb-check:$3 = -3
// gdb-command:print arg2
// gdb-check:$4 = 4.5
// gdb-command:print arg3
// gdb-check:$5 = 5
// gdb-command:continue


// === LLDB TESTS ==================================================================================

// lldb-command:run

// STRUCT
// lldb-command:print arg1
// lldb-check:[...]$0 = 1
// lldb-command:print arg2
// lldb-check:[...]$1 = 2
// lldb-command:continue

// ENUM
// lldb-command:print arg1
// lldb-check:[...]$2 = -3
// lldb-command:print arg2
// lldb-check:[...]$3 = 4.5
// lldb-command:print arg3
// lldb-check:[...]$4 = 5
// lldb-command:continue

#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

struct Struct {
    x: isize
}

impl Struct {

    fn static_method(arg1: isize, arg2: isize) -> isize {
        zzz(); // #break
        arg1 + arg2
    }
}

enum Enum {
    Variant1 { x: isize },
    Variant2,
    Variant3(f64, isize, char),
}

impl Enum {

    fn static_method(arg1: isize, arg2: f64, arg3: usize) -> isize {
        zzz(); // #break
        arg1
    }
}

fn main() {
    Struct::static_method(1, 2);
    Enum::static_method(-3, 4.5, 5);
}

fn zzz() {()}
