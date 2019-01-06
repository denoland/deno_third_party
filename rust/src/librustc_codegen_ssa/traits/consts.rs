use super::BackendTypes;
use mir::place::PlaceRef;
use rustc::mir::interpret::Allocation;
use rustc::mir::interpret::Scalar;
use rustc::ty::layout;
use syntax::symbol::LocalInternedString;

pub trait ConstMethods<'tcx>: BackendTypes {
    // Constant constructors
    fn const_null(&self, t: Self::Type) -> Self::Value;
    fn const_undef(&self, t: Self::Type) -> Self::Value;
    fn const_int(&self, t: Self::Type, i: i64) -> Self::Value;
    fn const_uint(&self, t: Self::Type, i: u64) -> Self::Value;
    fn const_uint_big(&self, t: Self::Type, u: u128) -> Self::Value;
    fn const_bool(&self, val: bool) -> Self::Value;
    fn const_i32(&self, i: i32) -> Self::Value;
    fn const_u32(&self, i: u32) -> Self::Value;
    fn const_u64(&self, i: u64) -> Self::Value;
    fn const_usize(&self, i: u64) -> Self::Value;
    fn const_u8(&self, i: u8) -> Self::Value;

    // This is a 'c-like' raw string, which differs from
    // our boxed-and-length-annotated strings.
    fn const_cstr(&self, s: LocalInternedString, null_terminated: bool) -> Self::Value;

    fn const_str_slice(&self, s: LocalInternedString) -> Self::Value;
    fn const_fat_ptr(&self, ptr: Self::Value, meta: Self::Value) -> Self::Value;
    fn const_struct(&self, elts: &[Self::Value], packed: bool) -> Self::Value;
    fn const_array(&self, ty: Self::Type, elts: &[Self::Value]) -> Self::Value;
    fn const_vector(&self, elts: &[Self::Value]) -> Self::Value;
    fn const_bytes(&self, bytes: &[u8]) -> Self::Value;

    fn const_get_elt(&self, v: Self::Value, idx: u64) -> Self::Value;
    fn const_get_real(&self, v: Self::Value) -> Option<(f64, bool)>;
    fn const_to_uint(&self, v: Self::Value) -> u64;
    fn const_to_opt_u128(&self, v: Self::Value, sign_ext: bool) -> Option<u128>;

    fn is_const_integral(&self, v: Self::Value) -> bool;
    fn is_const_real(&self, v: Self::Value) -> bool;

    fn scalar_to_backend(
        &self,
        cv: Scalar,
        layout: &layout::Scalar,
        llty: Self::Type,
    ) -> Self::Value;
    fn from_const_alloc(
        &self,
        layout: layout::TyLayout<'tcx>,
        alloc: &Allocation,
        offset: layout::Size,
    ) -> PlaceRef<'tcx, Self::Value>;

    fn const_ptrcast(&self, val: Self::Value, ty: Self::Type) -> Self::Value;
}
