// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Platform-dependent platform abstraction
//!
//! The `std::sys` module is the abstracted interface through which
//! `std` talks to the underlying operating system. It has different
//! implementations for different operating system families, today
//! just Unix and Windows, and initial support for Redox.
//!
//! The centralization of platform-specific code in this module is
//! enforced by the "platform abstraction layer" tidy script in
//! `tools/tidy/src/pal.rs`.
//!
//! This module is closely related to the platform-independent system
//! integration code in `std::sys_common`. See that module's
//! documentation for details.
//!
//! In the future it would be desirable for the independent
//! implementations of this module to be extracted to their own crates
//! that `std` can link to, thus enabling their implementation
//! out-of-tree via crate replacement. Though due to the complex
//! inter-dependencies within `std` that will be a challenging goal to
//! achieve.

#![allow(missing_debug_implementations)]

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        pub use self::unix::*;
    } else if #[cfg(windows)] {
        mod windows;
        pub use self::windows::*;
    } else if #[cfg(target_os = "cloudabi")] {
        mod cloudabi;
        pub use self::cloudabi::*;
    } else if #[cfg(target_os = "redox")] {
        mod redox;
        pub use self::redox::*;
    } else if #[cfg(target_arch = "wasm32")] {
        mod wasm;
        pub use self::wasm::*;
    } else {
        compile_error!("libstd doesn't compile for this platform yet");
    }
}

// Import essential modules from both platforms when documenting. These are
// then later used in the `std::os` module when documenting, for example,
// Windows when we're compiling for Linux.

#[cfg(dox)]
cfg_if! {
    if #[cfg(any(unix, target_os = "redox"))] {
        // On unix we'll document what's already available
        pub use self::ext as unix_ext;
    } else if #[cfg(any(target_os = "cloudabi", target_arch = "wasm32"))] {
        // On CloudABI and wasm right now the module below doesn't compile
        // (missing things in `libc` which is empty) so just omit everything
        // with an empty module
        #[unstable(issue = "0", feature = "std_internals")]
        pub mod unix_ext {}
    } else {
        // On other platforms like Windows document the bare bones of unix
        use os::linux as platform;
        #[path = "unix/ext/mod.rs"]
        pub mod unix_ext;
    }
}

#[cfg(dox)]
cfg_if! {
    if #[cfg(windows)] {
        // On windows we'll just be documenting what's already available
        pub use self::ext as windows_ext;
    } else if #[cfg(any(target_os = "cloudabi", target_arch = "wasm32"))] {
        // On CloudABI and wasm right now the shim below doesn't compile, so
        // just omit it
        #[unstable(issue = "0", feature = "std_internals")]
        pub mod windows_ext {}
    } else {
        // On all other platforms (aka linux/osx/etc) then pull in a "minimal"
        // amount of windows goop which ends up compiling
        #[macro_use]
        #[path = "windows/compat.rs"]
        mod compat;

        #[path = "windows/c.rs"]
        mod c;

        #[path = "windows/ext/mod.rs"]
        pub mod windows_ext;
    }
}
