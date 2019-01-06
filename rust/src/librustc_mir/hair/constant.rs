use syntax::ast;
use rustc::ty::{self, Ty, TyCtxt, ParamEnv};
use syntax_pos::symbol::Symbol;
use rustc::mir::interpret::{ConstValue, Scalar};

#[derive(PartialEq)]
crate enum LitToConstError {
    UnparseableFloat,
    Reported,
}

crate fn lit_to_const<'a, 'gcx, 'tcx>(
    lit: &'tcx ast::LitKind,
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    ty: Ty<'tcx>,
    neg: bool,
) -> Result<ty::Const<'tcx>, LitToConstError> {
    use syntax::ast::*;

    let trunc = |n| {
        let param_ty = ParamEnv::reveal_all().and(tcx.lift_to_global(&ty).unwrap());
        let width = tcx.layout_of(param_ty).map_err(|_| LitToConstError::Reported)?.size;
        trace!("trunc {} with size {} and shift {}", n, width.bits(), 128 - width.bits());
        let shift = 128 - width.bits();
        let result = (n << shift) >> shift;
        trace!("trunc result: {}", result);
        Ok(ConstValue::Scalar(Scalar::Bits {
            bits: result,
            size: width.bytes() as u8,
        }))
    };

    use rustc::mir::interpret::*;
    let lit = match *lit {
        LitKind::Str(ref s, _) => {
            let s = s.as_str();
            let id = tcx.allocate_bytes(s.as_bytes());
            ConstValue::new_slice(Scalar::Ptr(id.into()), s.len() as u64, &tcx)
        },
        LitKind::ByteStr(ref data) => {
            let id = tcx.allocate_bytes(data);
            ConstValue::Scalar(Scalar::Ptr(id.into()))
        },
        LitKind::Byte(n) => ConstValue::Scalar(Scalar::Bits {
            bits: n as u128,
            size: 1,
        }),
        LitKind::Int(n, _) if neg => {
            let n = n as i128;
            let n = n.overflowing_neg().0;
            trunc(n as u128)?
        },
        LitKind::Int(n, _) => trunc(n)?,
        LitKind::Float(n, fty) => {
            parse_float(n, fty, neg).map_err(|_| LitToConstError::UnparseableFloat)?
        }
        LitKind::FloatUnsuffixed(n) => {
            let fty = match ty.sty {
                ty::Float(fty) => fty,
                _ => bug!()
            };
            parse_float(n, fty, neg).map_err(|_| LitToConstError::UnparseableFloat)?
        }
        LitKind::Bool(b) => ConstValue::Scalar(Scalar::from_bool(b)),
        LitKind::Char(c) => ConstValue::Scalar(Scalar::from_char(c)),
    };
    Ok(ty::Const { val: lit, ty })
}

fn parse_float<'tcx>(
    num: Symbol,
    fty: ast::FloatTy,
    neg: bool,
) -> Result<ConstValue<'tcx>, ()> {
    let num = num.as_str();
    use rustc_apfloat::ieee::{Single, Double};
    use rustc_apfloat::Float;
    let (bits, size) = match fty {
        ast::FloatTy::F32 => {
            num.parse::<f32>().map_err(|_| ())?;
            let mut f = num.parse::<Single>().unwrap_or_else(|e| {
                panic!("apfloat::ieee::Single failed to parse `{}`: {:?}", num, e)
            });
            if neg {
                f = -f;
            }
            (f.to_bits(), 4)
        }
        ast::FloatTy::F64 => {
            num.parse::<f64>().map_err(|_| ())?;
            let mut f = num.parse::<Double>().unwrap_or_else(|e| {
                panic!("apfloat::ieee::Single failed to parse `{}`: {:?}", num, e)
            });
            if neg {
                f = -f;
            }
            (f.to_bits(), 8)
        }
    };

    Ok(ConstValue::Scalar(Scalar::Bits { bits, size }))
}
