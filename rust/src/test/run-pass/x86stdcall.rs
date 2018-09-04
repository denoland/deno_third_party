// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-wasm32-bare no libc to test ffi with

// GetLastError doesn't seem to work with stack switching

#[cfg(windows)]
mod kernel32 {
  extern "system" {
    pub fn SetLastError(err: usize);
    pub fn GetLastError() -> usize;
  }
}


#[cfg(windows)]
pub fn main() {
    unsafe {
        let expected = 1234;
        kernel32::SetLastError(expected);
        let actual = kernel32::GetLastError();
        println!("actual = {}", actual);
        assert_eq!(expected, actual);
    }
}

#[cfg(any(target_os = "android",
          target_os = "bitrig",
          target_os = "cloudabi",
          target_os = "dragonfly",
          target_os = "emscripten",
          target_os = "freebsd",
          target_os = "linux",
          target_os = "macos",
          target_os = "netbsd",
          target_os = "openbsd",
          target_os = "solaris"))]
pub fn main() { }
