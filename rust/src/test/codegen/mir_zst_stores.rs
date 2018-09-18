// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
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
use std::marker::PhantomData;

#[derive(Copy, Clone)]
struct Zst { phantom: PhantomData<Zst> }

// CHECK-LABEL: @mir
// CHECK-NOT: store{{.*}}undef
#[no_mangle]
pub fn mir() {
    let x = Zst { phantom: PhantomData };
    let y = (x, 0);
    drop(y);
    drop((0, x));
}
