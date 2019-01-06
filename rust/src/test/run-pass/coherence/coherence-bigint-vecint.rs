// run-pass
// aux-build:coherence_lib.rs
// revisions: old re

#![cfg_attr(re, feature(re_rebalance_coherence))]

// pretty-expanded FIXME #23616

extern crate coherence_lib as lib;
use lib::Remote1;

pub struct BigInt;

impl Remote1<BigInt> for Vec<isize> { }

fn main() { }
