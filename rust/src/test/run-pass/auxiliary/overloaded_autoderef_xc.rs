// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::Deref;

struct DerefWithHelper<H, T> {
    pub helper: H,
    pub value: Option<T>
}

trait Helper<T> {
    fn helper_borrow(&self) -> &T;
}

impl<T> Helper<T> for Option<T> {
    fn helper_borrow(&self) -> &T {
        self.as_ref().unwrap()
    }
}

impl<T, H: Helper<T>> Deref for DerefWithHelper<H, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.helper.helper_borrow()
    }
}

// Test cross-crate autoderef + vtable.
pub fn check<T: PartialEq>(x: T, y: T) -> bool {
    let d: DerefWithHelper<Option<T>, T> = DerefWithHelper { helper: Some(x), value: None };
    d.eq(&y)
}
