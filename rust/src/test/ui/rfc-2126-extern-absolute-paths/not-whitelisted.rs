// edition:2018

#![feature(uniform_paths)]

// Tests that arbitrary crates (other than `core`, `std` and `meta`)
// aren't allowed without `--extern`, even if they're in the sysroot.
use alloc; //~ ERROR unresolved import `alloc`
use test; //~ ERROR cannot import a built-in macro
use proc_macro; // OK, imports the built-in `proc_macro` attribute, but not the `proc_macro` crate.

fn main() {}
