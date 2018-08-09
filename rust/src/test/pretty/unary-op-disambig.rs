// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: --crate-type=lib

// Preserve semicolons that disambiguate unops

fn f() { }

fn block_semi() -> isize { { f() }; -1 }

fn block_nosemi() -> isize { ({ 0 }) - 1 }

fn if_semi() -> isize { if true { f() } else { f() }; -1 }

fn if_nosemi() -> isize { (if true { 0 } else { 0 }) - 1 }

fn alt_semi() -> isize { match true { true => { f() } _ => { } }; -1 }

fn alt_no_semi() -> isize { (match true { true => { 0 } _ => { 1 } }) - 1 }

fn stmt() { { f() }; -1; }
