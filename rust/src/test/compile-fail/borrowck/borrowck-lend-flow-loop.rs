// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Note: the borrowck analysis is currently flow-insensitive.
// Therefore, some of these errors are marked as spurious and could be
// corrected by a simple change to the analysis.  The others are
// either genuine or would require more advanced changes.  The latter
// cases are noted.

#![feature(box_syntax)]

fn borrow(_v: &isize) {}
fn borrow_mut(_v: &mut isize) {}
fn cond() -> bool { panic!() }
fn produce<T>() -> T { panic!(); }

fn inc(v: &mut Box<isize>) {
    *v = box (**v + 1);
}

fn loop_overarching_alias_mut() {
    // In this instance, the borrow encompasses the entire loop.

    let mut v: Box<_> = box 3;
    let mut x = &mut v;
    **x += 1;
    loop {
        borrow(&*v); //~ ERROR cannot borrow
    }
}

fn block_overarching_alias_mut() {
    // In this instance, the borrow encompasses the entire closure call.

    let mut v: Box<_> = box 3;
    let mut x = &mut v;
    for _ in 0..3 {
        borrow(&*v); //~ ERROR cannot borrow
    }
    *x = box 5;
}

fn loop_aliased_mut() {
    // In this instance, the borrow is carried through the loop.

    let mut v: Box<_> = box 3;
    let mut w: Box<_> = box 4;
    let mut _x = &w;
    loop {
        borrow_mut(&mut *v); //~ ERROR cannot borrow
        _x = &v;
    }
}

fn while_aliased_mut() {
    // In this instance, the borrow is carried through the loop.

    let mut v: Box<_> = box 3;
    let mut w: Box<_> = box 4;
    let mut _x = &w;
    while cond() {
        borrow_mut(&mut *v); //~ ERROR cannot borrow
        _x = &v;
    }
}


fn loop_aliased_mut_break() {
    // In this instance, the borrow is carried through the loop.

    let mut v: Box<_> = box 3;
    let mut w: Box<_> = box 4;
    let mut _x = &w;
    loop {
        borrow_mut(&mut *v);
        _x = &v;
        break;
    }
    borrow_mut(&mut *v); //~ ERROR cannot borrow
}

fn while_aliased_mut_break() {
    // In this instance, the borrow is carried through the loop.

    let mut v: Box<_> = box 3;
    let mut w: Box<_> = box 4;
    let mut _x = &w;
    while cond() {
        borrow_mut(&mut *v);
        _x = &v;
        break;
    }
    borrow_mut(&mut *v); //~ ERROR cannot borrow
}

fn while_aliased_mut_cond(cond: bool, cond2: bool) {
    let mut v: Box<_> = box 3;
    let mut w: Box<_> = box 4;
    let mut x = &mut w;
    while cond {
        **x += 1;
        borrow(&*v); //~ ERROR cannot borrow
        if cond2 {
            x = &mut v; //~ ERROR cannot borrow
        }
    }
}

fn loop_break_pops_scopes<'r, F>(_v: &'r mut [usize], mut f: F) where
    F: FnMut(&'r mut usize) -> bool,
{
    // Here we check that when you break out of an inner loop, the
    // borrows that go out of scope as you exit the inner loop are
    // removed from the bitset.

    while cond() {
        while cond() {
            // this borrow is limited to the scope of `r`...
            let r: &'r mut usize = produce();
            if !f(&mut *r) {
                break; // ...so it is not live as exit the `while` loop here
            }
        }
    }
}

fn loop_loop_pops_scopes<'r, F>(_v: &'r mut [usize], mut f: F)
    where F: FnMut(&'r mut usize) -> bool
{
    // Similar to `loop_break_pops_scopes` but for the `loop` keyword

    while cond() {
        while cond() {
            // this borrow is limited to the scope of `r`...
            let r: &'r mut usize = produce();
            if !f(&mut *r) {
                continue; // ...so it is not live as exit (and re-enter) the `while` loop here
            }
        }
    }
}

fn main() {}
