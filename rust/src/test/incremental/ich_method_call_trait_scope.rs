// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Check that the hash for a method call is sensitive to the traits in
// scope.

// revisions: rpass1 rpass2
// compile-flags: -Z query-dep-graph

#![feature(rustc_attrs)]

fn test<T>() { }

trait Trait1 {
    fn method(&self) { }
}

impl Trait1 for () { }

trait Trait2 {
    fn method(&self) { }
}

impl Trait2 for () { }

mod mod3 {
    #[cfg(rpass1)]
    use Trait1;
    #[cfg(rpass2)]
    use Trait2;

    #[rustc_clean(label="Hir", cfg="rpass2")]
    #[rustc_clean(label="HirBody", cfg="rpass2")]
    #[rustc_dirty(label="TypeckTables", cfg="rpass2")]
    fn bar() {
        ().method();
    }

    #[rustc_clean(label="Hir", cfg="rpass2")]
    #[rustc_clean(label="HirBody", cfg="rpass2")]
    #[rustc_clean(label="TypeckTables", cfg="rpass2")]
    fn baz() {
        22; // no method call, traits in scope don't matter
    }
}

fn main() { }
