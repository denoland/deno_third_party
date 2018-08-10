// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test a case where we are trying to prove `'x: 'y` and are forced to
// approximate the shorter end-point (`'y`) to with `'static`. This is
// because `'y` is higher-ranked but we know of no relations to other
// regions. Note that `'static` shows up in the stderr output as `'0`.
//
// FIXME(#45827) Because of shortcomings in the MIR type checker,
// these errors are not (yet) reported.

// compile-flags:-Zborrowck=mir -Zverbose

#![feature(rustc_attrs)]

use std::cell::Cell;

// Callee knows that:
//
// 'x: 'a
//
// so the only way we can ensure that `'x: 'y` is to show that
// `'a: 'static`.
fn establish_relationships<'a, 'b, F>(_cell_a: &Cell<&'a u32>, _cell_b: &Cell<&'b u32>, _closure: F)
where
    F: for<'x, 'y> FnMut(
        &Cell<&'a &'x u32>, // shows that 'x: 'a
        &Cell<&'x u32>,
        &Cell<&'y u32>,
    ),
{
}

fn demand_y<'x, 'y>(_cell_x: &Cell<&'x u32>, _cell_y: &Cell<&'y u32>, _y: &'y u32) {}

#[rustc_regions]
fn supply<'a, 'b>(cell_a: Cell<&'a u32>, cell_b: Cell<&'b u32>) {
    establish_relationships(&cell_a, &cell_b, |_outlives, x, y| {
        //~^ ERROR does not outlive free region

        // Only works if 'x: 'y:
        demand_y(x, y, x.get()) //~ WARNING not reporting region error due to nll
    });
}

fn main() {}
