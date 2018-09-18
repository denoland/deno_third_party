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
// gdb-command:print string1.length
// gdb-check:$1 = 48
// gdb-command:print string2.length
// gdb-check:$2 = 49
// gdb-command:print string3.length
// gdb-check:$3 = 50
// gdb-command:continue


// === LLDB TESTS ==================================================================================

// lldb-command:run

// lldb-command:print string1.length
// lldb-check:[...]$0 = 48
// lldb-command:print string2.length
// lldb-check:[...]$1 = 49
// lldb-command:print string3.length
// lldb-check:[...]$2 = 50

// lldb-command:continue

#![allow(unused_variables)]
#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

// This test case makes sure that debug info does not ICE when include_str is
// used multiple times (see issue #11322).

fn main() {
    let string1 = include_str!("text-to-include-1.txt");
    let string2 = include_str!("text-to-include-2.txt");
    let string3 = include_str!("text-to-include-3.txt");

    zzz(); // #break
}

fn zzz() {()}
