// compile-flags: -C no-prepopulate-passes

#![crate_type="lib"]
#![feature(repr_simd)]

use std::marker::PhantomData;

pub struct Zst1;
pub struct Zst2(());

#[repr(transparent)]
pub struct F32(f32);

// CHECK: define float @test_F32(float %arg0)
#[no_mangle]
pub extern fn test_F32(_: F32) -> F32 { loop {} }

#[repr(transparent)]
pub struct Ptr(*mut u8);

// CHECK: define i8* @test_Ptr(i8* %arg0)
#[no_mangle]
pub extern fn test_Ptr(_: Ptr) -> Ptr { loop {} }

#[repr(transparent)]
pub struct WithZst(u64, Zst1);

// CHECK: define i64 @test_WithZst(i64 %arg0)
#[no_mangle]
pub extern fn test_WithZst(_: WithZst) -> WithZst { loop {} }

#[repr(transparent)]
pub struct WithZeroSizedArray(*const f32, [i8; 0]);

// Apparently we use i32* when newtype-unwrapping f32 pointers. Whatever.
// CHECK: define i32* @test_WithZeroSizedArray(i32* %arg0)
#[no_mangle]
pub extern fn test_WithZeroSizedArray(_: WithZeroSizedArray) -> WithZeroSizedArray { loop {} }

#[repr(transparent)]
pub struct Generic<T>(T);

// CHECK: define double @test_Generic(double %arg0)
#[no_mangle]
pub extern fn test_Generic(_: Generic<f64>) -> Generic<f64> { loop {} }

#[repr(transparent)]
pub struct GenericPlusZst<T>(T, Zst2);

#[repr(u8)]
pub enum Bool { True, False, FileNotFound }

// CHECK: define{{( zeroext)?}} i8 @test_Gpz(i8{{( zeroext)?}} %arg0)
#[no_mangle]
pub extern fn test_Gpz(_: GenericPlusZst<Bool>) -> GenericPlusZst<Bool> { loop {} }

#[repr(transparent)]
pub struct LifetimePhantom<'a, T: 'a>(*const T, PhantomData<&'a T>);

// CHECK: define i16* @test_LifetimePhantom(i16* %arg0)
#[no_mangle]
pub extern fn test_LifetimePhantom(_: LifetimePhantom<i16>) -> LifetimePhantom<i16> { loop {} }

// This works despite current alignment resrictions because PhantomData is always align(1)
#[repr(transparent)]
pub struct UnitPhantom<T, U> { val: T, unit: PhantomData<U> }

pub struct Px;

// CHECK: define float @test_UnitPhantom(float %arg0)
#[no_mangle]
pub extern fn test_UnitPhantom(_: UnitPhantom<f32, Px>) -> UnitPhantom<f32, Px> { loop {} }

#[repr(transparent)]
pub struct TwoZsts(Zst1, i8, Zst2);

// CHECK: define{{( signext)?}} i8 @test_TwoZsts(i8{{( signext)?}} %arg0)
#[no_mangle]
pub extern fn test_TwoZsts(_: TwoZsts) -> TwoZsts { loop {} }

#[repr(transparent)]
pub struct Nested1(Zst2, Generic<f64>);

// CHECK: define double @test_Nested1(double %arg0)
#[no_mangle]
pub extern fn test_Nested1(_: Nested1) -> Nested1 { loop {} }

#[repr(transparent)]
pub struct Nested2(Nested1, Zst1);

// CHECK: define double @test_Nested2(double %arg0)
#[no_mangle]
pub extern fn test_Nested2(_: Nested2) -> Nested2 { loop {} }

#[repr(simd)]
struct f32x4(f32, f32, f32, f32);

#[repr(transparent)]
pub struct Vector(f32x4);

// CHECK: define <4 x float> @test_Vector(<4 x float> %arg0)
#[no_mangle]
pub extern fn test_Vector(_: Vector) -> Vector { loop {} }

trait Mirror { type It: ?Sized; }
impl<T: ?Sized> Mirror for T { type It = Self; }

#[repr(transparent)]
pub struct StructWithProjection(<f32 as Mirror>::It);

// CHECK: define float @test_Projection(float %arg0)
#[no_mangle]
pub extern fn test_Projection(_: StructWithProjection) -> StructWithProjection { loop {} }


// All that remains to be tested are aggregates. They are tested in separate files called repr-
// transparent-*.rs  with `only-*` or `ignore-*` directives, because the expected LLVM IR
// function signatures vary so much that it's not reasonably possible to cover all of them with a
// single CHECK line.
//
// You may be wondering why we don't just compare the return types and argument types for equality
// with FileCheck regex captures. Well, rustc doesn't perform newtype unwrapping on newtypes
// containing aggregates. This is OK on all ABIs we support, but because LLVM has not gotten rid of
// pointee types yet, the IR function signature will be syntactically different (%Foo* vs
// %FooWrapper*).
