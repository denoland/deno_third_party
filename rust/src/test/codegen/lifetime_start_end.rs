// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags: -O -C no-prepopulate-passes

#![crate_type = "lib"]

// CHECK-LABEL: @test
#[no_mangle]
pub fn test() {
    let a = 0;
    &a; // keep variable in an alloca

// CHECK: [[S_a:%[0-9]+]] = bitcast i32* %a to i8*
// CHECK: call void @llvm.lifetime.start{{.*}}(i{{[0-9 ]+}}, i8* [[S_a]])

    {
        let b = &Some(a);
        &b; // keep variable in an alloca

// CHECK: [[S_b:%[0-9]+]] = bitcast { i32, i32 }** %b to i8*
// CHECK: call void @llvm.lifetime.start{{.*}}(i{{[0-9 ]+}}, i8* [[S_b]])

// CHECK: [[S__4:%[0-9]+]] = bitcast { i32, i32 }* %_4 to i8*
// CHECK: call void @llvm.lifetime.start{{.*}}(i{{[0-9 ]+}}, i8* [[S__4]])

// CHECK: [[E_b:%[0-9]+]] = bitcast { i32, i32 }** %b to i8*
// CHECK: call void @llvm.lifetime.end{{.*}}(i{{[0-9 ]+}}, i8* [[E_b]])

// CHECK: [[E__4:%[0-9]+]] = bitcast { i32, i32 }* %_4 to i8*
// CHECK: call void @llvm.lifetime.end{{.*}}(i{{[0-9 ]+}}, i8* [[E__4]])
    }

    let c = 1;
    &c; // keep variable in an alloca

// CHECK: [[S_c:%[0-9]+]] = bitcast i32* %c to i8*
// CHECK: call void @llvm.lifetime.start{{.*}}(i{{[0-9 ]+}}, i8* [[S_c]])

// CHECK: [[E_c:%[0-9]+]] = bitcast i32* %c to i8*
// CHECK: call void @llvm.lifetime.end{{.*}}(i{{[0-9 ]+}}, i8* [[E_c]])

// CHECK: [[E_a:%[0-9]+]] = bitcast i32* %a to i8*
// CHECK: call void @llvm.lifetime.end{{.*}}(i{{[0-9 ]+}}, i8* [[E_a]])
}
