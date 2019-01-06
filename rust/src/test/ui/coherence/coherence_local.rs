// Test that we are able to introduce a negative constraint that
// `MyType: !MyTrait` along with other "fundamental" wrappers.

// aux-build:coherence_copy_like_lib.rs
// compile-pass
// skip-codegen
// revisions: old re

#![cfg_attr(re, feature(re_rebalance_coherence))]
#![allow(dead_code)]

extern crate coherence_copy_like_lib as lib;

struct MyType { x: i32 }

// These are all legal because they are all fundamental types:

impl lib::MyCopy for MyType { }
impl<'a> lib::MyCopy for &'a MyType { }
impl<'a> lib::MyCopy for &'a Box<MyType> { }
impl lib::MyCopy for Box<MyType> { }
impl lib::MyCopy for lib::MyFundamentalStruct<MyType> { }
impl lib::MyCopy for lib::MyFundamentalStruct<Box<MyType>> { }


fn main() { }
