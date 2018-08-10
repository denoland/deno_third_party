// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Calculation and management of a Strict Version Hash for crates
//!
//! The SVH is used for incremental compilation to track when HIR
//! nodes have changed between compilations, and also to detect
//! mismatches where we have two versions of the same crate that were
//! compiled from distinct sources.

use std::fmt;
use std::hash::{Hash, Hasher};
use serialize::{Encodable, Decodable, Encoder, Decoder};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Svh {
    hash: u64,
}

impl Svh {
    /// Create a new `Svh` given the hash. If you actually want to
    /// compute the SVH from some HIR, you want the `calculate_svh`
    /// function found in `librustc_incremental`.
    pub fn new(hash: u64) -> Svh {
        Svh { hash: hash }
    }

    pub fn as_u64(&self) -> u64 {
        self.hash
    }

    pub fn to_string(&self) -> String {
        format!("{:016x}", self.hash)
    }
}

impl Hash for Svh {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.hash.to_le().hash(state);
    }
}

impl fmt::Display for Svh {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&self.to_string())
    }
}

impl Encodable for Svh {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_u64(self.as_u64().to_le())
    }
}

impl Decodable for Svh {
    fn decode<D: Decoder>(d: &mut D) -> Result<Svh, D::Error> {
        d.read_u64()
         .map(u64::from_le)
         .map(Svh::new)
    }
}

impl_stable_hash_for!(struct Svh {
    hash
});
