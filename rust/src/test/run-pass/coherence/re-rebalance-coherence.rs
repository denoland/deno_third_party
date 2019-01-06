#![allow(dead_code)]
#![feature(re_rebalance_coherence)]

// run-pass
// aux-build:re_rebalance_coherence_lib.rs

extern crate re_rebalance_coherence_lib as lib;
use lib::*;

struct Oracle;
impl Backend for Oracle {}
impl<'a, T:'a, Tab> QueryFragment<Oracle> for BatchInsert<'a, T, Tab> {}

fn main() {}
