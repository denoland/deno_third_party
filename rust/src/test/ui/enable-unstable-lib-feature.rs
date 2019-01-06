// Test that enabling an unstable feature disables warnings

// aux-build:stability_cfg2.rs

#![feature(unstable_test_feature)]
#![deny(non_snake_case)] // To trigger a hard error

// Shouldn't generate a warning about unstable features
#[allow(unused_extern_crates)]
extern crate stability_cfg2;

pub fn BOGUS() { } //~ ERROR

pub fn main() { }
