// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that all coercions can actually be done using casts (modulo the lints).

#![allow(trivial_casts, trivial_numeric_casts)]

trait Foo {
    fn foo(&self) {}
}

pub struct Bar;

impl Foo for Bar {}

pub fn main() {
    // Numeric
    let _ = 42_i32 as i32;
    let _ = 42_u8 as u8;

    // & to * pointers
    let x: &u32 = &42;
    let _ = x as *const u32;

    let x: &mut u32 = &mut 42;
    let _ = x as *mut u32;

    // unsize array
    let x: &[u32; 3] = &[42, 43, 44];
    let _ = x as &[u32];
    let _ = x as *const [u32];

    let x: &mut [u32; 3] = &mut [42, 43, 44];
    let _ = x as &mut [u32];
    let _ = x as *mut [u32];

    let x: Box<[u32; 3]> = Box::new([42, 43, 44]);
    let _ = x as Box<[u32]>;

    // unsize trait
    let x: &Bar = &Bar;
    let _ = x as &Foo;
    let _ = x as *const Foo;

    let x: &mut Bar = &mut Bar;
    let _ = x as &mut Foo;
    let _ = x as *mut Foo;

    let x: Box<Bar> = Box::new(Bar);
    let _ = x as Box<Foo>;

    // functions
    fn baz(_x: i32) {}
    let _ = &baz as &Fn(i32);
    let x = |_x: i32| {};
    let _ = &x as &Fn(i32);
}

// subtyping
pub fn test_subtyping<'a, 'b: 'a>(a: &'a Bar, b: &'b Bar) {
    let _ = a as &'a Bar;
    let _ = b as &'a Bar;
    let _ = b as &'b Bar;
}
