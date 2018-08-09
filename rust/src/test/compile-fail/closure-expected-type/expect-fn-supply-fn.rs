// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

fn with_closure_expecting_fn_with_free_region<F>(_: F)
    where F: for<'a> FnOnce(fn(&'a u32), &i32)
{
}

fn with_closure_expecting_fn_with_bound_region<F>(_: F)
    where F: FnOnce(fn(&u32), &i32)
{
}

fn expect_free_supply_free_from_fn<'x>(x: &'x u32) {
    // Here, the type given for `'x` "obscures" a region from the
    // expected signature that is bound at closure level.
    with_closure_expecting_fn_with_free_region(|x: fn(&'x u32), y| {});
    //~^ ERROR mismatched types
    //~| ERROR mismatched types
}

fn expect_free_supply_free_from_closure() {
    // A variant on the previous test. Here, the region `'a` will be
    // bound at the closure level, just as is expected, so no error
    // results.
    type Foo<'a> = fn(&'a u32);
    with_closure_expecting_fn_with_free_region(|_x: Foo<'_>, y| {});
}

fn expect_free_supply_bound() {
    // Here, we are given a function whose region is bound at closure level,
    // but we expect one bound in the argument. Error results.
    with_closure_expecting_fn_with_free_region(|x: fn(&u32), y| {});
    //~^ ERROR type mismatch in closure arguments
}

fn expect_bound_supply_free_from_fn<'x>(x: &'x u32) {
    // Here, we are given a `fn(&u32)` but we expect a `fn(&'x
    // u32)`. In principle, this could be ok, but we demand equality.
    with_closure_expecting_fn_with_bound_region(|x: fn(&'x u32), y| {});
    //~^ ERROR type mismatch in closure arguments
}

fn expect_bound_supply_free_from_closure() {
    // A variant on the previous test. Here, the region `'a` will be
    // bound at the closure level, but we expect something bound at
    // the argument level.
    type Foo<'a> = fn(&'a u32);
    with_closure_expecting_fn_with_bound_region(|_x: Foo<'_>, y| {});
    //~^ ERROR type mismatch in closure arguments
}

fn expect_bound_supply_bound<'x>(x: &'x u32) {
    // No error in this case. The supplied type supplies the bound
    // regions, and hence we are able to figure out the type of `y`
    // from the expected type
    with_closure_expecting_fn_with_bound_region(|x: for<'z> fn(&'z u32), y| {
    });
}

fn main() { }
