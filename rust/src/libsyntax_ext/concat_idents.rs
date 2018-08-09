// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use syntax::ast;
use syntax::ext::base::*;
use syntax::ext::base;
use syntax::feature_gate;
use syntax::parse::token;
use syntax::ptr::P;
use syntax_pos::Span;
use syntax_pos::symbol::Symbol;
use syntax::tokenstream::TokenTree;

pub fn expand_syntax_ext<'cx>(cx: &'cx mut ExtCtxt,
                              sp: Span,
                              tts: &[TokenTree])
                              -> Box<base::MacResult + 'cx> {
    if !cx.ecfg.enable_concat_idents() {
        feature_gate::emit_feature_err(&cx.parse_sess,
                                       "concat_idents",
                                       sp,
                                       feature_gate::GateIssue::Language,
                                       feature_gate::EXPLAIN_CONCAT_IDENTS);
        return base::DummyResult::expr(sp);
    }

    if tts.is_empty() {
        cx.span_err(sp, "concat_idents! takes 1 or more arguments.");
        return DummyResult::expr(sp);
    }

    let mut res_str = String::new();
    for (i, e) in tts.iter().enumerate() {
        if i & 1 == 1 {
            match *e {
                TokenTree::Token(_, token::Comma) => {}
                _ => {
                    cx.span_err(sp, "concat_idents! expecting comma.");
                    return DummyResult::expr(sp);
                }
            }
        } else {
            match *e {
                TokenTree::Token(_, token::Ident(ident, _)) =>
                    res_str.push_str(&ident.as_str()),
                _ => {
                    cx.span_err(sp, "concat_idents! requires ident args.");
                    return DummyResult::expr(sp);
                }
            }
        }
    }

    let ident = ast::Ident::new(Symbol::intern(&res_str), sp.apply_mark(cx.current_expansion.mark));

    struct ConcatIdentsResult { ident: ast::Ident }

    impl base::MacResult for ConcatIdentsResult {
        fn make_expr(self: Box<Self>) -> Option<P<ast::Expr>> {
            Some(P(ast::Expr {
                id: ast::DUMMY_NODE_ID,
                node: ast::ExprKind::Path(None, ast::Path::from_ident(self.ident)),
                span: self.ident.span,
                attrs: ast::ThinVec::new(),
            }))
        }

        fn make_ty(self: Box<Self>) -> Option<P<ast::Ty>> {
            Some(P(ast::Ty {
                id: ast::DUMMY_NODE_ID,
                node: ast::TyKind::Path(None, ast::Path::from_ident(self.ident)),
                span: self.ident.span,
            }))
        }
    }

    Box::new(ConcatIdentsResult { ident })
}
