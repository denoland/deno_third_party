// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-cross-compile

#![feature(quote, rustc_private)]

extern crate syntax;
extern crate syntax_pos;

use syntax::ast;
use syntax::codemap::FilePathMapping;
use syntax::print::pprust;
use syntax::symbol::Symbol;
use syntax_pos::DUMMY_SP;

fn main() {
    let ps = syntax::parse::ParseSess::new(FilePathMapping::empty());
    let mut resolver = syntax::ext::base::DummyResolver;
    let mut cx = syntax::ext::base::ExtCtxt::new(
        &ps,
        syntax::ext::expand::ExpansionConfig::default("qquote".to_string()),
        &mut resolver);
    let cx = &mut cx;

    assert_eq!(pprust::expr_to_string(&*quote_expr!(&cx, 23)), "23");

    let expr = quote_expr!(&cx, 2 - $abcd + 7); //~ ERROR cannot find value `abcd` in this scope
    assert_eq!(pprust::expr_to_string(&*expr), "2 - $abcd + 7");
}
