// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -C no-prepopulate-passes

#![crate_type = "lib"]
#![feature(repr_packed)]

#[repr(packed)]
pub struct Packed1 {
    dealign: u8,
    data: u32
}

#[repr(packed(2))]
pub struct Packed2 {
    dealign: u8,
    data: u32
}

// CHECK-LABEL: @write_pkd1
#[no_mangle]
pub fn write_pkd1(pkd: &mut Packed1) -> u32 {
// CHECK: %{{.*}} = load i32, i32* %{{.*}}, align 1
// CHECK: store i32 42, i32* %{{.*}}, align 1
    let result = pkd.data;
    pkd.data = 42;
    result
}

// CHECK-LABEL: @write_pkd2
#[no_mangle]
pub fn write_pkd2(pkd: &mut Packed2) -> u32 {
// CHECK: %{{.*}} = load i32, i32* %{{.*}}, align 2
// CHECK: store i32 42, i32* %{{.*}}, align 2
    let result = pkd.data;
    pkd.data = 42;
    result
}

pub struct Array([i32; 8]);
#[repr(packed)]
pub struct BigPacked1 {
    dealign: u8,
    data: Array
}

#[repr(packed(2))]
pub struct BigPacked2 {
    dealign: u8,
    data: Array
}

// CHECK-LABEL: @call_pkd1
#[no_mangle]
pub fn call_pkd1(f: fn() -> Array) -> BigPacked1 {
// CHECK: [[ALLOCA:%[_a-z0-9]+]] = alloca %Array
// CHECK: call void %{{.*}}(%Array* noalias nocapture sret dereferenceable(32) [[ALLOCA]])
// CHECK: call void @llvm.memcpy.{{.*}}(i8* %{{.*}}, i8* %{{.*}}, i{{[0-9]+}} 32, i32 1, i1 false)
    // check that calls whose destination is a field of a packed struct
    // go through an alloca rather than calling the function with an
    // unaligned destination.
    BigPacked1 { dealign: 0, data: f() }
}

// CHECK-LABEL: @call_pkd2
#[no_mangle]
pub fn call_pkd2(f: fn() -> Array) -> BigPacked2 {
// CHECK: [[ALLOCA:%[_a-z0-9]+]] = alloca %Array
// CHECK: call void %{{.*}}(%Array* noalias nocapture sret dereferenceable(32) [[ALLOCA]])
// CHECK: call void @llvm.memcpy.{{.*}}(i8* %{{.*}}, i8* %{{.*}}, i{{[0-9]+}} 32, i32 2, i1 false)
    // check that calls whose destination is a field of a packed struct
    // go through an alloca rather than calling the function with an
    // unaligned destination.
    BigPacked2 { dealign: 0, data: f() }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Packed1Pair(u8, u32);

#[repr(packed(2))]
#[derive(Copy, Clone)]
pub struct Packed2Pair(u8, u32);

// CHECK-LABEL: @pkd1_pair
#[no_mangle]
pub fn pkd1_pair(pair1: &mut Packed1Pair, pair2: &mut Packed1Pair) {
// CHECK: call void @llvm.memcpy.{{.*}}(i8* %{{.*}}, i8* %{{.*}}, i{{[0-9]+}} 5, i32 1, i1 false)
    *pair2 = *pair1;
}

// CHECK-LABEL: @pkd2_pair
#[no_mangle]
pub fn pkd2_pair(pair1: &mut Packed2Pair, pair2: &mut Packed2Pair) {
// CHECK: call void @llvm.memcpy.{{.*}}(i8* %{{.*}}, i8* %{{.*}}, i{{[0-9]+}} 6, i32 2, i1 false)
    *pair2 = *pair1;
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Packed1NestedPair((u32, u32));

#[repr(packed(2))]
#[derive(Copy, Clone)]
pub struct Packed2NestedPair((u32, u32));

// CHECK-LABEL: @pkd1_nested_pair
#[no_mangle]
pub fn pkd1_nested_pair(pair1: &mut Packed1NestedPair, pair2: &mut Packed1NestedPair) {
// CHECK: call void @llvm.memcpy.{{.*}}(i8* %{{.*}}, i8* %{{.*}}, i{{[0-9]+}} 8, i32 1, i1 false)
    *pair2 = *pair1;
}

// CHECK-LABEL: @pkd2_nested_pair
#[no_mangle]
pub fn pkd2_nested_pair(pair1: &mut Packed2NestedPair, pair2: &mut Packed2NestedPair) {
// CHECK: call void @llvm.memcpy.{{.*}}(i8* %{{.*}}, i8* %{{.*}}, i{{[0-9]+}} 8, i32 2, i1 false)
    *pair2 = *pair1;
}

