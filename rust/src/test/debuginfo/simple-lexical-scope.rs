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

// gdb-command:print x
// gdb-check:$1 = false
// gdb-command:continue

// gdb-command:print x
// gdb-check:$2 = false
// gdb-command:continue

// gdb-command:print x
// gdb-check:$3 = 10
// gdb-command:continue

// gdb-command:print x
// gdb-check:$4 = 10
// gdb-command:continue

// gdb-command:print x
// gdb-check:$5 = 10.5
// gdb-command:continue

// gdb-command:print x
// gdb-check:$6 = 10
// gdb-command:continue

// gdb-command:print x
// gdb-check:$7 = false
// gdb-command:continue


// === LLDB TESTS ==================================================================================

// lldb-command:run

// lldb-command:print x
// lldb-check:[...]$0 = false
// lldb-command:continue

// lldb-command:print x
// lldb-check:[...]$1 = false
// lldb-command:continue

// lldb-command:print x
// lldb-check:[...]$2 = 10
// lldb-command:continue

// lldb-command:print x
// lldb-check:[...]$3 = 10
// lldb-command:continue

// lldb-command:print x
// lldb-check:[...]$4 = 10.5
// lldb-command:continue

// lldb-command:print x
// lldb-check:[...]$5 = 10
// lldb-command:continue

// lldb-command:print x
// lldb-check:[...]$6 = false
// lldb-command:continue


#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

fn main() {
    let x = false;

    zzz(); // #break
    sentinel();

    {
        zzz(); // #break
        sentinel();

        let x = 10;

        zzz(); // #break
        sentinel();

        {
            zzz(); // #break
            sentinel();

            let x = 10.5f64;

            zzz(); // #break
            sentinel();
        }

        zzz(); // #break
        sentinel();
    }

    zzz(); // #break
    sentinel();
}

fn zzz() {()}
fn sentinel() {()}
