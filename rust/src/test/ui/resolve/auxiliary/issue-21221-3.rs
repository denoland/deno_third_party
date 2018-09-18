// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// testing whether the lookup mechanism picks up types
// defined in the outside crate

#![crate_type="lib"]

pub mod outer {
    // should suggest this
    pub trait OuterTrait {}

    // should not suggest this since the module is private
    mod private_module {
        pub trait OuterTrait {}
    }

    // should not suggest since the trait is private
    pub mod public_module {
        trait OuterTrait {}
    }
}
