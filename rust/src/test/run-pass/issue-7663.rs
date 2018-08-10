// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#![allow(unused_imports, dead_code)]

mod test1 {

    mod foo { pub fn p() -> isize { 1 } }
    mod bar { pub fn p() -> isize { 2 } }

    pub mod baz {
        use test1::bar::p;

        pub fn my_main() { assert_eq!(p(), 2); }
    }
}

mod test2 {

    mod foo { pub fn p() -> isize { 1 } }
    mod bar { pub fn p() -> isize { 2 } }

    pub mod baz {
        use test2::bar::p;

        pub fn my_main() { assert_eq!(p(), 2); }
    }
}

fn main() {
    test1::baz::my_main();
    test2::baz::my_main();
}
