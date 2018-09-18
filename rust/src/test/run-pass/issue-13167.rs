// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

use std::slice;

pub struct PhfMapEntries<'a, T: 'a> {
    iter: slice::Iter<'a, (&'static str, T)>,
}

impl<'a, T> Iterator for PhfMapEntries<'a, T> {
    type Item = (&'static str, &'a T);

    fn next(&mut self) -> Option<(&'static str, &'a T)> {
        self.iter.by_ref().map(|&(key, ref value)| (key, value)).next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

fn main() {}
