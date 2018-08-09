// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(lang_items, no_core, optin_builtin_traits)]
#![no_core]

#[lang="copy"]
trait Copy { }

#[lang="sized"]
trait Sized { }

#[lang = "freeze"]
auto trait Freeze {}

#[lang="start"]
fn start(_main: *const u8, _argc: isize, _argv: *const *const u8) -> isize { 0 }

extern {
    fn _foo() -> [u8; 16];
}

fn _main() {
    let _a = unsafe { _foo() };
}
