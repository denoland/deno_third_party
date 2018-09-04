// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-cross-compile


// The general idea of this test is to enumerate all "interesting" expressions and check that
// `parse(print(e)) == e` for all `e`.  Here's what's interesting, for the purposes of this test:
//
//  1. The test focuses on expression nesting, because interactions between different expression
//     types are harder to test manually than single expression types in isolation.
//
//  2. The test only considers expressions of at most two nontrivial nodes.  So it will check `x +
//     x` and `x + (x - x)` but not `(x * x) + (x - x)`.  The assumption here is that the correct
//     handling of an expression might depend on the expression's parent, but doesn't depend on its
//     siblings or any more distant ancestors.
//
// 3. The test only checks certain expression kinds.  The assumption is that similar expression
//    types, such as `if` and `while` or `+` and `-`,  will be handled identically in the printer
//    and parser.  So if all combinations of exprs involving `if` work correctly, then combinations
//    using `while`, `if let`, and so on will likely work as well.


#![feature(rustc_private)]

extern crate syntax;

use syntax::ast::*;
use syntax::codemap::{Spanned, DUMMY_SP, FileName};
use syntax::codemap::FilePathMapping;
use syntax::fold::{self, Folder};
use syntax::parse::{self, ParseSess};
use syntax::print::pprust;
use syntax::ptr::P;
use syntax::util::ThinVec;


fn parse_expr(ps: &ParseSess, src: &str) -> P<Expr> {
    let mut p = parse::new_parser_from_source_str(ps,
                                                  FileName::Custom("expr".to_owned()),
                                                  src.to_owned());
    p.parse_expr().unwrap()
}


// Helper functions for building exprs
fn expr(kind: ExprKind) -> P<Expr> {
    P(Expr {
        id: DUMMY_NODE_ID,
        node: kind,
        span: DUMMY_SP,
        attrs: ThinVec::new(),
    })
}

fn make_x() -> P<Expr> {
    let seg = PathSegment::from_ident(Ident::from_str("x"));
    let path = Path { segments: vec![seg], span: DUMMY_SP };
    expr(ExprKind::Path(None, path))
}

/// Iterate over exprs of depth up to `depth`.  The goal is to explore all "interesting"
/// combinations of expression nesting.  For example, we explore combinations using `if`, but not
/// `while` or `match`, since those should print and parse in much the same way as `if`.
fn iter_exprs(depth: usize, f: &mut FnMut(P<Expr>)) {
    if depth == 0 {
        f(make_x());
        return;
    }

    let mut g = |e| f(expr(e));

    for kind in 0 .. 16 {
        match kind {
            0 => iter_exprs(depth - 1, &mut |e| g(ExprKind::Box(e))),
            1 => iter_exprs(depth - 1, &mut |e| g(ExprKind::Call(e, vec![]))),
            2 => {
                let seg = PathSegment::from_ident(Ident::from_str("x"));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::MethodCall(
                            seg.clone(), vec![e, make_x()])));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::MethodCall(
                            seg.clone(), vec![make_x(), e])));
            },
            3 => {
                let op = Spanned { span: DUMMY_SP, node: BinOpKind::Add };
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Binary(op, e, make_x())));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Binary(op, make_x(), e)));
            },
            4 => {
                let op = Spanned { span: DUMMY_SP, node: BinOpKind::Mul };
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Binary(op, e, make_x())));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Binary(op, make_x(), e)));
            },
            5 => {
                let op = Spanned { span: DUMMY_SP, node: BinOpKind::Shl };
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Binary(op, e, make_x())));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Binary(op, make_x(), e)));
            },
            6 => {
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Unary(UnOp::Deref, e)));
            },
            7 => {
                let block = P(Block {
                    stmts: Vec::new(),
                    id: DUMMY_NODE_ID,
                    rules: BlockCheckMode::Default,
                    span: DUMMY_SP,
                    recovered: false,
                });
                iter_exprs(depth - 1, &mut |e| g(ExprKind::If(e, block.clone(), None)));
            },
            8 => {
                let decl = P(FnDecl {
                    inputs: vec![],
                    output: FunctionRetTy::Default(DUMMY_SP),
                    variadic: false,
                });
                iter_exprs(depth - 1, &mut |e| g(
                        ExprKind::Closure(CaptureBy::Value,
                                          Movability::Movable,
                                          decl.clone(),
                                          e,
                                          DUMMY_SP)));
            },
            9 => {
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Assign(e, make_x())));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Assign(make_x(), e)));
            },
            10 => {
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Field(e, Ident::from_str("f"))));
            },
            11 => {
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Range(
                            Some(e), Some(make_x()), RangeLimits::HalfOpen)));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Range(
                            Some(make_x()), Some(e), RangeLimits::HalfOpen)));
            },
            12 => {
                iter_exprs(depth - 1, &mut |e| g(ExprKind::AddrOf(Mutability::Immutable, e)));
            },
            13 => {
                g(ExprKind::Ret(None));
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Ret(Some(e))));
            },
            14 => {
                let path = Path::from_ident(Ident::from_str("S"));
                g(ExprKind::Struct(path, vec![], Some(make_x())));
            },
            15 => {
                iter_exprs(depth - 1, &mut |e| g(ExprKind::Try(e)));
            },
            _ => panic!("bad counter value in iter_exprs"),
        }
    }
}


// Folders for manipulating the placement of `Paren` nodes.  See below for why this is needed.

/// Folder that removes all `ExprKind::Paren` nodes.
struct RemoveParens;

impl Folder for RemoveParens {
    fn fold_expr(&mut self, e: P<Expr>) -> P<Expr> {
        let e = match e.node {
            ExprKind::Paren(ref inner) => inner.clone(),
            _ => e.clone(),
        };
        e.map(|e| fold::noop_fold_expr(e, self))
    }
}


/// Folder that inserts `ExprKind::Paren` nodes around every `Expr`.
struct AddParens;

impl Folder for AddParens {
    fn fold_expr(&mut self, e: P<Expr>) -> P<Expr> {
        let e = e.map(|e| fold::noop_fold_expr(e, self));
        P(Expr {
            id: DUMMY_NODE_ID,
            node: ExprKind::Paren(e),
            span: DUMMY_SP,
            attrs: ThinVec::new(),
        })
    }
}

fn main() {
    syntax::with_globals(|| run());
}

fn run() {
    let ps = ParseSess::new(FilePathMapping::empty());

    iter_exprs(2, &mut |e| {
        // If the pretty printer is correct, then `parse(print(e))` should be identical to `e`,
        // modulo placement of `Paren` nodes.
        let printed = pprust::expr_to_string(&e);
        println!("printed: {}", printed);

        let parsed = parse_expr(&ps, &printed);

        // We want to know if `parsed` is structurally identical to `e`, ignoring trivial
        // differences like placement of `Paren`s or the exact ranges of node spans.
        // Unfortunately, there is no easy way to make this comparison.  Instead, we add `Paren`s
        // everywhere we can, then pretty-print.  This should give an unambiguous representation of
        // each `Expr`, and it bypasses nearly all of the parenthesization logic, so we aren't
        // relying on the correctness of the very thing we're testing.
        let e1 = AddParens.fold_expr(RemoveParens.fold_expr(e));
        let text1 = pprust::expr_to_string(&e1);
        let e2 = AddParens.fold_expr(RemoveParens.fold_expr(parsed));
        let text2 = pprust::expr_to_string(&e2);
        assert!(text1 == text2,
                "exprs are not equal:\n  e =      {:?}\n  parsed = {:?}",
                text1, text2);
    });
}
