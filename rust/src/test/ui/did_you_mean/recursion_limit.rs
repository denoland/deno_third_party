// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test that the recursion limit can be changed and that the compiler
// suggests a fix. In this case, we have deeply nested types that will
// fail the `Send` check by overflow when the recursion limit is set
// very low.

#![allow(dead_code)]
#![recursion_limit="10"]

macro_rules! link {
    ($id:ident, $t:ty) => {
        enum $id { $id($t) }
    }
}

link! { A, B }
link! { B, C }
link! { C, D }
link! { D, E }
link! { E, F }
link! { F, G }
link! { G, H }
link! { H, I }
link! { I, J }
link! { J, K }
link! { K, L }
link! { L, M }
link! { M, N }

enum N { N(usize) }

fn is_send<T:Send>() { }

fn main() {
    is_send::<A>(); //~ ERROR overflow evaluating the requirement
}
