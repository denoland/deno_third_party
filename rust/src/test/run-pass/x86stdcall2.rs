// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(std_misc)]

pub type HANDLE = usize;
pub type DWORD = u32;
pub type SIZE_T = u32;
pub type LPVOID = usize;
pub type BOOL = u8;

#[cfg(windows)]
mod kernel32 {
    use super::{HANDLE, DWORD, SIZE_T, LPVOID, BOOL};

    extern "system" {
        pub fn GetProcessHeap() -> HANDLE;
        pub fn HeapAlloc(hHeap: HANDLE, dwFlags: DWORD, dwBytes: SIZE_T)
                      -> LPVOID;
        pub fn HeapFree(hHeap: HANDLE, dwFlags: DWORD, lpMem: LPVOID) -> BOOL;
    }
}


#[cfg(windows)]
pub fn main() {
    let heap = unsafe { kernel32::GetProcessHeap() };
    let mem = unsafe { kernel32::HeapAlloc(heap, 0, 100) };
    assert!(mem != 0);
    let res = unsafe { kernel32::HeapFree(heap, 0, mem) };
    assert!(res != 0);
}

#[cfg(not(windows))]
pub fn main() { }
