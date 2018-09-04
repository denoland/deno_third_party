// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![forbid(non_camel_case_types)]
#![allow(dead_code)]

struct ONE_TWO_THREE;
//~^ ERROR type `ONE_TWO_THREE` should have a camel case name such as `OneTwoThree`

struct foo { //~ ERROR type `foo` should have a camel case name such as `Foo`
    bar: isize,
}

enum foo2 { //~ ERROR type `foo2` should have a camel case name such as `Foo2`
    Bar
}

struct foo3 { //~ ERROR type `foo3` should have a camel case name such as `Foo3`
    bar: isize
}

type foo4 = isize; //~ ERROR type `foo4` should have a camel case name such as `Foo4`

enum Foo5 {
    bar //~ ERROR variant `bar` should have a camel case name such as `Bar`
}

trait foo6 { //~ ERROR trait `foo6` should have a camel case name such as `Foo6`
    fn dummy(&self) { }
}

fn f<ty>(_: ty) {} //~ ERROR type parameter `ty` should have a camel case name such as `Ty`

#[repr(C)]
struct foo7 {
    bar: isize,
}

type __ = isize; //~ ERROR type `__` should have a camel case name such as `CamelCase`

struct X86_64;

struct X86__64; //~ ERROR type `X86__64` should have a camel case name such as `X86_64`

struct Abc_123; //~ ERROR type `Abc_123` should have a camel case name such as `Abc123`

struct A1_b2_c3; //~ ERROR type `A1_b2_c3` should have a camel case name such as `A1B2C3`

fn main() { }
