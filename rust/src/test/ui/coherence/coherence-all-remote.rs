// aux-build:coherence_lib.rs
// revisions: old re

#![cfg_attr(re, feature(re_rebalance_coherence))]

extern crate coherence_lib as lib;
use lib::Remote1;

impl<T> Remote1<T> for isize { }
//[old]~^ ERROR E0210
//[re]~^^ ERROR E0210

fn main() { }
