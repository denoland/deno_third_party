use hair::*;
use hair::cx::Cx;
use hair::cx::to_ref::ToRef;
use rustc::middle::region;
use rustc::hir;

use rustc_data_structures::indexed_vec::Idx;

impl<'tcx> Mirror<'tcx> for &'tcx hir::Block {
    type Output = Block<'tcx>;

    fn make_mirror<'a, 'gcx>(self, cx: &mut Cx<'a, 'gcx, 'tcx>) -> Block<'tcx> {
        // We have to eagerly lower the "spine" of the statements
        // in order to get the lexical scoping correctly.
        let stmts = mirror_stmts(cx, self.hir_id.local_id, &*self.stmts);
        let opt_destruction_scope =
            cx.region_scope_tree.opt_destruction_scope(self.hir_id.local_id);
        Block {
            targeted_by_break: self.targeted_by_break,
            region_scope: region::Scope {
                id: self.hir_id.local_id,
                data: region::ScopeData::Node
            },
            opt_destruction_scope,
            span: self.span,
            stmts,
            expr: self.expr.to_ref(),
            safety_mode: match self.rules {
                hir::BlockCheckMode::DefaultBlock =>
                    BlockSafety::Safe,
                hir::BlockCheckMode::UnsafeBlock(..) =>
                    BlockSafety::ExplicitUnsafe(self.id),
                hir::BlockCheckMode::PushUnsafeBlock(..) =>
                    BlockSafety::PushUnsafe,
                hir::BlockCheckMode::PopUnsafeBlock(..) =>
                    BlockSafety::PopUnsafe
            },
        }
    }
}

fn mirror_stmts<'a, 'gcx, 'tcx>(cx: &mut Cx<'a, 'gcx, 'tcx>,
                                block_id: hir::ItemLocalId,
                                stmts: &'tcx [hir::Stmt])
                                -> Vec<StmtRef<'tcx>> {
    let mut result = vec![];
    for (index, stmt) in stmts.iter().enumerate() {
        let hir_id = cx.tcx.hir().node_to_hir_id(stmt.node.id());
        let opt_dxn_ext = cx.region_scope_tree.opt_destruction_scope(hir_id.local_id);
        let stmt_span = StatementSpan(cx.tcx.hir().span(stmt.node.id()));
        match stmt.node {
            hir::StmtKind::Expr(ref expr, _) |
            hir::StmtKind::Semi(ref expr, _) => {
                result.push(StmtRef::Mirror(Box::new(Stmt {
                    kind: StmtKind::Expr {
                        scope: region::Scope {
                            id: hir_id.local_id,
                            data: region::ScopeData::Node
                        },
                        expr: expr.to_ref(),
                    },
                    opt_destruction_scope: opt_dxn_ext,
                    span: stmt_span,
                })))
            }
            hir::StmtKind::Decl(ref decl, _) => {
                match decl.node {
                    hir::DeclKind::Item(..) => {
                        // ignore for purposes of the MIR
                    }
                    hir::DeclKind::Local(ref local) => {
                        let remainder_scope = region::Scope {
                            id: block_id,
                            data: region::ScopeData::Remainder(
                                region::FirstStatementIndex::new(index)),
                        };

                        let mut pattern = cx.pattern_from_hir(&local.pat);

                        if let Some(ty) = &local.ty {
                            if let Some(&user_ty) = cx.tables.user_provided_types().get(ty.hir_id) {
                                debug!("mirror_stmts: user_ty={:?}", user_ty);
                                pattern = Pattern {
                                    ty: pattern.ty,
                                    span: pattern.span,
                                    kind: Box::new(PatternKind::AscribeUserType {
                                        user_ty: PatternTypeProjection::from_user_type(user_ty),
                                        user_ty_span: ty.span,
                                        subpattern: pattern
                                    })
                                };
                            }
                        }

                        result.push(StmtRef::Mirror(Box::new(Stmt {
                            kind: StmtKind::Let {
                                remainder_scope: remainder_scope,
                                init_scope: region::Scope {
                                    id: hir_id.local_id,
                                    data: region::ScopeData::Node
                                },
                                pattern,
                                initializer: local.init.to_ref(),
                                lint_level: cx.lint_level_of(local.id),
                            },
                            opt_destruction_scope: opt_dxn_ext,
                            span: stmt_span,
                        })));
                    }
                }
            }
        }
    }
    return result;
}

pub fn to_expr_ref<'a, 'gcx, 'tcx>(cx: &mut Cx<'a, 'gcx, 'tcx>,
                                   block: &'tcx hir::Block)
                                   -> ExprRef<'tcx> {
    let block_ty = cx.tables().node_id_to_type(block.hir_id);
    let temp_lifetime = cx.region_scope_tree.temporary_scope(block.hir_id.local_id);
    let expr = Expr {
        ty: block_ty,
        temp_lifetime,
        span: block.span,
        kind: ExprKind::Block { body: block },
    };
    expr.to_ref()
}
