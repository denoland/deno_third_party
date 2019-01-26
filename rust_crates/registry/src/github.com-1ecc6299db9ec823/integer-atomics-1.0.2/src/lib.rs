//! This crate allows you to compile code that needs the unstable integer atomics types
//! (`Atomic{U,I}{8,16,32,64}`) with the stable compiler.
//!
//! If the `nightly` feature is enabled, it simply re-exports these types from `std::sync::atomic`.
//!
//! Otherwise, they are emulated with the existing stable `AtomicUsize` and compare-exchange loops.
//! Because of that, the `Atomic{U,I}64` types are only available on 64-bit platforms.
//! Also, this is obviously much slower than real atomics. This is only a stopgap solution until
//! those are finally stabilized.
//!
//! This crate has no documentation as these types are documented within the standard library docs.

#![cfg_attr(feature = "nightly", feature(integer_atomics))]

#[cfg(feature = "nightly")]
use std::sync::atomic;

#[cfg(not(feature = "nightly"))]
mod atomic;

pub use atomic::{AtomicI8, AtomicU8, AtomicI16, AtomicU16, AtomicI32, AtomicU32};
#[cfg(target_pointer_width = "64")]
pub use atomic::{AtomicI64, AtomicU64};

