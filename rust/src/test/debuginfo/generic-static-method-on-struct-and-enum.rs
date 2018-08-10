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


#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

struct Struct {
    x: isize
}

impl Struct {

    fn static_method<T1, T2>(arg1: T1, arg2: T2) -> isize {
        zzz(); // #break
        return 0;
    }
}

enum Enum {
    Variant1 { x: isize },
    Variant2,
    Variant3(f64, isize, char),
}

impl Enum {

    fn static_method<T1, T2, T3>(arg1: T1, arg2: T2, arg3: T3) -> isize {
        zzz(); // #break
        return 1;
    }
}

fn main() {
    Struct::static_method(1, 2);
    Enum::static_method(-3, 4.5f64, 5);
}

fn zzz() {()}
