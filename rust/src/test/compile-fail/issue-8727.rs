// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// error-pattern:reached the recursion limit while instantiating `generic::<std::option::Option<

// Verify the compiler fails with an error on infinite function
// recursions.

fn generic<T>() {
    generic::<Option<T>>();
}


fn main () {
    // Use generic<T> at least once to trigger instantiation.
    generic::<i32>();
}
