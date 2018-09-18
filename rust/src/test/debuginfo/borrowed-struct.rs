// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags:-g
// min-lldb-version: 310

// === GDB TESTS ===================================================================================

// gdb-command:run

// gdb-command:print *stack_val_ref
// gdbg-check:$1 = {x = 10, y = 23.5}
// gdbr-check:$1 = borrowed_struct::SomeStruct {x: 10, y: 23.5}

// gdb-command:print *stack_val_interior_ref_1
// gdb-check:$2 = 10

// gdb-command:print *stack_val_interior_ref_2
// gdb-check:$3 = 23.5

// gdb-command:print *ref_to_unnamed
// gdbg-check:$4 = {x = 11, y = 24.5}
// gdbr-check:$4 = borrowed_struct::SomeStruct {x: 11, y: 24.5}

// gdb-command:print *unique_val_ref
// gdbg-check:$5 = {x = 13, y = 26.5}
// gdbr-check:$5 = borrowed_struct::SomeStruct {x: 13, y: 26.5}

// gdb-command:print *unique_val_interior_ref_1
// gdb-check:$6 = 13

// gdb-command:print *unique_val_interior_ref_2
// gdb-check:$7 = 26.5


// === LLDB TESTS ==================================================================================

// lldb-command:run

// lldb-command:print *stack_val_ref
// lldb-check:[...]$0 = SomeStruct { x: 10, y: 23.5 }

// lldb-command:print *stack_val_interior_ref_1
// lldb-check:[...]$1 = 10

// lldb-command:print *stack_val_interior_ref_2
// lldb-check:[...]$2 = 23.5

// lldb-command:print *ref_to_unnamed
// lldb-check:[...]$3 = SomeStruct { x: 11, y: 24.5 }

// lldb-command:print *unique_val_ref
// lldb-check:[...]$4 = SomeStruct { x: 13, y: 26.5 }

// lldb-command:print *unique_val_interior_ref_1
// lldb-check:[...]$5 = 13

// lldb-command:print *unique_val_interior_ref_2
// lldb-check:[...]$6 = 26.5

#![allow(unused_variables)]
#![feature(box_syntax)]
#![feature(omit_gdb_pretty_printer_section)]
#![omit_gdb_pretty_printer_section]

struct SomeStruct {
    x: isize,
    y: f64
}

fn main() {
    let stack_val: SomeStruct = SomeStruct { x: 10, y: 23.5 };
    let stack_val_ref: &SomeStruct = &stack_val;
    let stack_val_interior_ref_1: &isize = &stack_val.x;
    let stack_val_interior_ref_2: &f64 = &stack_val.y;
    let ref_to_unnamed: &SomeStruct = &SomeStruct { x: 11, y: 24.5 };

    let unique_val: Box<_> = box SomeStruct { x: 13, y: 26.5 };
    let unique_val_ref: &SomeStruct = &*unique_val;
    let unique_val_interior_ref_1: &isize = &unique_val.x;
    let unique_val_interior_ref_2: &f64 = &unique_val.y;

    zzz(); // #break
}

fn zzz() {()}
