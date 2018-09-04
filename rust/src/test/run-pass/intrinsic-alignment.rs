// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-wasm32-bare seems not important to test here

#![feature(intrinsics, main)]

mod rusti {
    extern "rust-intrinsic" {
        pub fn pref_align_of<T>() -> usize;
        pub fn min_align_of<T>() -> usize;
    }
}

#[cfg(any(target_os = "android",
          target_os = "cloudabi",
          target_os = "dragonfly",
          target_os = "emscripten",
          target_os = "freebsd",
          target_os = "linux",
          target_os = "macos",
          target_os = "netbsd",
          target_os = "openbsd",
          target_os = "solaris"))]
mod m {
    #[main]
    #[cfg(target_arch = "x86")]
    pub fn main() {
        unsafe {
            assert_eq!(::rusti::pref_align_of::<u64>(), 8);
            assert_eq!(::rusti::min_align_of::<u64>(), 4);
        }
    }

    #[main]
    #[cfg(not(target_arch = "x86"))]
    pub fn main() {
        unsafe {
            assert_eq!(::rusti::pref_align_of::<u64>(), 8);
            assert_eq!(::rusti::min_align_of::<u64>(), 8);
        }
    }
}

#[cfg(target_os = "bitrig")]
mod m {
    #[main]
    #[cfg(target_arch = "x86_64")]
    pub fn main() {
        unsafe {
            assert_eq!(::rusti::pref_align_of::<u64>(), 8);
            assert_eq!(::rusti::min_align_of::<u64>(), 8);
        }
    }
}

#[cfg(target_os = "windows")]
mod m {
    #[main]
    #[cfg(target_arch = "x86")]
    pub fn main() {
        unsafe {
            assert_eq!(::rusti::pref_align_of::<u64>(), 8);
            assert_eq!(::rusti::min_align_of::<u64>(), 8);
        }
    }

    #[main]
    #[cfg(target_arch = "x86_64")]
    pub fn main() {
        unsafe {
            assert_eq!(::rusti::pref_align_of::<u64>(), 8);
            assert_eq!(::rusti::min_align_of::<u64>(), 8);
        }
    }
}
