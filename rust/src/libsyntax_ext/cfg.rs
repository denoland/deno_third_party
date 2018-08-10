// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// The compiler code necessary to support the cfg! extension, which expands to
/// a literal `true` or `false` based on whether the given cfg matches the
/// current compilation environment.

use syntax::ext::base::*;
use syntax::ext::base;
use syntax::ext::build::AstBuilder;
use syntax::attr;
use syntax::tokenstream;
use syntax::parse::token;
use syntax_pos::Span;

pub fn expand_cfg<'cx>(cx: &mut ExtCtxt,
                       sp: Span,
                       tts: &[tokenstream::TokenTree])
                       -> Box<base::MacResult + 'static> {
    let sp = sp.apply_mark(cx.current_expansion.mark);
    let mut p = cx.new_parser_from_tts(tts);
    let cfg = panictry!(p.parse_meta_item());

    let _ = p.eat(&token::Comma);

    if !p.eat(&token::Eof) {
        cx.span_err(sp, "expected 1 cfg-pattern");
        return DummyResult::expr(sp);
    }

    let matches_cfg = attr::cfg_matches(&cfg, cx.parse_sess, cx.ecfg.features);
    MacEager::expr(cx.expr_bool(sp, matches_cfg))
}
