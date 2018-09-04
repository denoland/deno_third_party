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
// compile-flags:-Zprint-mono-items=lazy -Zincremental=tmp/partitioning-tests/local-drop-glue
// compile-flags:-Zinline-in-all-cgus

#![allow(dead_code)]
#![crate_type="rlib"]

//~ MONO_ITEM fn core::ptr[0]::drop_in_place[0]<local_drop_glue::Struct[0]> @@ local_drop_glue[Internal] local_drop_glue-mod1[Internal]
struct Struct {
    _a: u32
}

impl Drop for Struct {
    //~ MONO_ITEM fn local_drop_glue::{{impl}}[0]::drop[0] @@ local_drop_glue[External]
    fn drop(&mut self) {}
}

//~ MONO_ITEM fn core::ptr[0]::drop_in_place[0]<local_drop_glue::Outer[0]> @@ local_drop_glue[Internal]
struct Outer {
    _a: Struct
}

//~ MONO_ITEM fn local_drop_glue::user[0] @@ local_drop_glue[External]
pub fn user()
{
    let _ = Outer {
        _a: Struct {
            _a: 0
        }
    };
}

pub mod mod1
{
    use super::Struct;

    //~ MONO_ITEM fn core::ptr[0]::drop_in_place[0]<local_drop_glue::mod1[0]::Struct2[0]> @@ local_drop_glue-mod1[Internal]
    struct Struct2 {
        _a: Struct,
        //~ MONO_ITEM fn core::ptr[0]::drop_in_place[0]<(u32, local_drop_glue::Struct[0])> @@ local_drop_glue-mod1[Internal]
        _b: (u32, Struct),
    }

    //~ MONO_ITEM fn local_drop_glue::mod1[0]::user[0] @@ local_drop_glue-mod1[External]
    pub fn user()
    {
        let _ = Struct2 {
            _a: Struct { _a: 0 },
            _b: (0, Struct { _a: 0 }),
        };
    }
}
