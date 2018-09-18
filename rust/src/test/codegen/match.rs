// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -C no-prepopulate-passes

#![crate_type = "lib"]

pub enum E {
    A,
    B,
}

// CHECK-LABEL: @exhaustive_match
#[no_mangle]
pub fn exhaustive_match(e: E, unit: ()) {
// CHECK: switch{{.*}}, label %[[OTHERWISE:[a-zA-Z0-9_]+]] [
// CHECK-NEXT: i[[TY:[0-9]+]] [[DISCR:[0-9]+]], label %[[A:[a-zA-Z0-9_]+]]
// CHECK-NEXT: i[[TY:[0-9]+]] [[DISCR:[0-9]+]], label %[[B:[a-zA-Z0-9_]+]]
// CHECK-NEXT: ]
// CHECK: [[A]]:
// CHECK-NEXT: br label %[[EXIT:[a-zA-Z0-9_]+]]
// CHECK: [[B]]:
// CHECK-NEXT: br label %[[EXIT:[a-zA-Z0-9_]+]]
// CHECK: [[OTHERWISE]]:
// CHECK-NEXT: unreachable
    match e {
        E::A => unit,
        E::B => unit,
    }
}
