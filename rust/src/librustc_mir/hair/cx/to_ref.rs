use hair::*;

use rustc::hir;
use syntax::ptr::P;

pub trait ToRef {
    type Output;
    fn to_ref(self) -> Self::Output;
}

impl<'a, 'tcx: 'a> ToRef for &'tcx hir::Expr {
    type Output = ExprRef<'tcx>;

    fn to_ref(self) -> ExprRef<'tcx> {
        ExprRef::Hair(self)
    }
}

impl<'a, 'tcx: 'a> ToRef for &'tcx P<hir::Expr> {
    type Output = ExprRef<'tcx>;

    fn to_ref(self) -> ExprRef<'tcx> {
        ExprRef::Hair(&**self)
    }
}

impl<'a, 'tcx: 'a> ToRef for Expr<'tcx> {
    type Output = ExprRef<'tcx>;

    fn to_ref(self) -> ExprRef<'tcx> {
        ExprRef::Mirror(Box::new(self))
    }
}

impl<'a, 'tcx: 'a, T, U> ToRef for &'tcx Option<T>
    where &'tcx T: ToRef<Output = U>
{
    type Output = Option<U>;

    fn to_ref(self) -> Option<U> {
        self.as_ref().map(|expr| expr.to_ref())
    }
}

impl<'a, 'tcx: 'a, T, U> ToRef for &'tcx Vec<T>
    where &'tcx T: ToRef<Output = U>
{
    type Output = Vec<U>;

    fn to_ref(self) -> Vec<U> {
        self.iter().map(|expr| expr.to_ref()).collect()
    }
}

impl<'a, 'tcx: 'a, T, U> ToRef for &'tcx P<[T]>
    where &'tcx T: ToRef<Output = U>
{
    type Output = Vec<U>;

    fn to_ref(self) -> Vec<U> {
        self.iter().map(|expr| expr.to_ref()).collect()
    }
}
