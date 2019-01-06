//! In general, there are a number of things for which it's convenient
//! to just call `builder.into` and have it emit its result into a
//! given location. This is basically for expressions or things that can be
//! wrapped up as expressions (e.g., blocks). To make this ergonomic, we use this
//! latter `EvalInto` trait.

use build::{BlockAnd, Builder};
use hair::*;
use rustc::mir::*;

pub(in build) trait EvalInto<'tcx> {
    fn eval_into<'a, 'gcx>(self,
                           builder: &mut Builder<'a, 'gcx, 'tcx>,
                           destination: &Place<'tcx>,
                           block: BasicBlock)
                           -> BlockAnd<()>;
}

impl<'a, 'gcx, 'tcx> Builder<'a, 'gcx, 'tcx> {
    pub fn into<E>(&mut self,
                   destination: &Place<'tcx>,
                   block: BasicBlock,
                   expr: E)
                   -> BlockAnd<()>
        where E: EvalInto<'tcx>
    {
        expr.eval_into(self, destination, block)
    }
}

impl<'tcx> EvalInto<'tcx> for ExprRef<'tcx> {
    fn eval_into<'a, 'gcx>(self,
                           builder: &mut Builder<'a, 'gcx, 'tcx>,
                           destination: &Place<'tcx>,
                           block: BasicBlock)
                           -> BlockAnd<()> {
        let expr = builder.hir.mirror(self);
        builder.into_expr(destination, block, expr)
    }
}

impl<'tcx> EvalInto<'tcx> for Expr<'tcx> {
    fn eval_into<'a, 'gcx>(self,
                           builder: &mut Builder<'a, 'gcx, 'tcx>,
                           destination: &Place<'tcx>,
                           block: BasicBlock)
                           -> BlockAnd<()> {
        builder.into_expr(destination, block, self)
    }
}
