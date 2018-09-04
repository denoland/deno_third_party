// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-tidy-linelength
// We specify -Z incremental here because we want to test the partitioning for
// incremental compilation
// compile-flags:-Zprint-mono-items=lazy -Zincremental=tmp/partitioning-tests/local-transitive-inlining
// compile-flags:-Zinline-in-all-cgus

#![allow(dead_code)]
#![crate_type="rlib"]

mod inline {

    //~ MONO_ITEM fn local_transitive_inlining::inline[0]::inlined_function[0] @@ local_transitive_inlining-indirect_user[Internal]
    #[inline(always)]
    pub fn inlined_function()
    {

    }
}

mod direct_user {
    use super::inline;

    //~ MONO_ITEM fn local_transitive_inlining::direct_user[0]::foo[0] @@ local_transitive_inlining-indirect_user[Internal]
    #[inline(always)]
    pub fn foo() {
        inline::inlined_function();
    }
}

pub mod indirect_user {
    use super::direct_user;

    //~ MONO_ITEM fn local_transitive_inlining::indirect_user[0]::bar[0] @@ local_transitive_inlining-indirect_user[External]
    pub fn bar() {
        direct_user::foo();
    }
}

pub mod non_user {

    //~ MONO_ITEM fn local_transitive_inlining::non_user[0]::baz[0] @@ local_transitive_inlining-non_user[External]
    pub fn baz() {

    }
}
