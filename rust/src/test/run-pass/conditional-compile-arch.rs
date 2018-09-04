// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pretty-expanded FIXME #23616

#[cfg(target_arch = "x86")]
pub fn main() { }

#[cfg(target_arch = "x86_64")]
pub fn main() { }

#[cfg(target_arch = "arm")]
pub fn main() { }

#[cfg(target_arch = "aarch64")]
pub fn main() { }

#[cfg(target_arch = "mips")]
pub fn main() { }

#[cfg(target_arch = "mips64")]
pub fn main() { }

#[cfg(target_arch = "powerpc")]
pub fn main() { }

#[cfg(target_arch = "powerpc64")]
pub fn main() { }

#[cfg(target_arch = "s390x")]
pub fn main() { }

#[cfg(target_arch = "asmjs")]
pub fn main() { }

#[cfg(target_arch = "wasm32")]
pub fn main() { }

#[cfg(target_arch = "sparc64")]
pub fn main() { }
