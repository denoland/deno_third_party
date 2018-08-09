// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # The Unicode Library
//!
//! Unicode-intensive functions for `char` and `str` types.
//!
//! This crate provides a collection of Unicode-related functionality,
//! including decompositions, conversions, etc., and provides traits
//! implementing these functions for the `char` and `str` types.
//!
//! The functionality included here is only that which is necessary to
//! provide for basic string-related manipulations. This crate does not
//! (yet) aim to provide a full set of Unicode tables.

#![unstable(feature = "unicode", issue = "27783")]
#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://doc.rust-lang.org/favicon.ico",
       html_root_url = "https://doc.rust-lang.org/nightly/",
       html_playground_url = "https://play.rust-lang.org/",
       issue_tracker_base_url = "https://github.com/rust-lang/rust/issues/",
       test(no_crate_inject, attr(allow(unused_variables), deny(warnings))))]
#![no_std]

#![feature(unicode_internals)]
#![feature(staged_api)]
#![rustc_deprecated(since = "1.27.0", reason = "moved into libcore")]

pub use core::unicode::*;
