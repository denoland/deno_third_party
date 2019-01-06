use infer::canonical::{Canonical, Canonicalized, CanonicalizedQueryResponse, QueryResponse};
use traits::query::Fallible;
use ty::{ParamEnvAnd, Ty, TyCtxt};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Subtype<'tcx> {
    pub sub: Ty<'tcx>,
    pub sup: Ty<'tcx>,
}

impl<'tcx> Subtype<'tcx> {
    pub fn new(sub: Ty<'tcx>, sup: Ty<'tcx>) -> Self {
        Self {
            sub,
            sup,
        }
    }
}

impl<'gcx: 'tcx, 'tcx> super::QueryTypeOp<'gcx, 'tcx> for Subtype<'tcx> {
    type QueryResponse = ();

    fn try_fast_path(_tcx: TyCtxt<'_, 'gcx, 'tcx>, key: &ParamEnvAnd<'tcx, Self>) -> Option<()> {
        if key.value.sub == key.value.sup {
            Some(())
        } else {
            None
        }
    }

    fn perform_query(
        tcx: TyCtxt<'_, 'gcx, 'tcx>,
        canonicalized: Canonicalized<'gcx, ParamEnvAnd<'tcx, Self>>,
    ) -> Fallible<CanonicalizedQueryResponse<'gcx, ()>> {
        tcx.type_op_subtype(canonicalized)
    }

    fn shrink_to_tcx_lifetime(
        v: &'a CanonicalizedQueryResponse<'gcx, ()>,
    ) -> &'a Canonical<'tcx, QueryResponse<'tcx, ()>> {
        v
    }
}

BraceStructTypeFoldableImpl! {
    impl<'tcx> TypeFoldable<'tcx> for Subtype<'tcx> {
        sub,
        sup,
    }
}

BraceStructLiftImpl! {
    impl<'a, 'tcx> Lift<'tcx> for Subtype<'a> {
        type Lifted = Subtype<'tcx>;
        sub,
        sup,
    }
}

impl_stable_hash_for! {
    struct Subtype<'tcx> { sub, sup }
}
