// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.



// Regression test for issue #374

// pretty-expanded FIXME #23616

enum sty { ty_nil, }

struct RawT {struct_: sty, cname: Option<String>, hash: usize}

fn mk_raw_ty(st: sty, cname: Option<String>) -> RawT {
    return RawT {struct_: st, cname: cname, hash: 0};
}

pub fn main() { mk_raw_ty(sty::ty_nil, None::<String>); }
