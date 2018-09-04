// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! "compile-fail/svh-uta-trait.rs" is checking that we detect a
//! change from `use foo::TraitB` to use `foo::TraitB` in the hash
//! (SVH) computation (#14132), since that will affect method
//! resolution.
//!
//! This is the upstream crate.

#![crate_name = "uta"]

mod traits {
    pub trait TraitA { fn val(&self) -> isize { 2 } }
    pub trait TraitB { fn val(&self) -> isize { 3 } }
}

impl traits::TraitA for () {}
impl traits::TraitB for () {}

pub fn foo<T>(_: isize) -> isize {
    use traits::TraitA;
    let v = ();
    v.val()
}
