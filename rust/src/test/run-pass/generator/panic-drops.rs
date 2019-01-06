// run-pass

// ignore-wasm32-bare compiled as panic=abort by default

#![feature(generators, generator_trait)]

use std::ops::Generator;
use std::panic;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

static A: AtomicUsize = ATOMIC_USIZE_INIT;

struct B;

impl Drop for B {
    fn drop(&mut self) {
        A.fetch_add(1, Ordering::SeqCst);
    }
}

fn bool_true() -> bool {
    true
}

fn main() {
    let b = B;
    let mut foo = || {
        if bool_true() {
            panic!();
        }
        drop(b);
        yield;
    };

    assert_eq!(A.load(Ordering::SeqCst), 0);
    let res = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        unsafe { foo.resume() }
    }));
    assert!(res.is_err());
    assert_eq!(A.load(Ordering::SeqCst), 1);

    let mut foo = || {
        if bool_true() {
            panic!();
        }
        drop(B);
        yield;
    };

    assert_eq!(A.load(Ordering::SeqCst), 1);
    let res = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        unsafe { foo.resume() }
    }));
    assert!(res.is_err());
    assert_eq!(A.load(Ordering::SeqCst), 1);
}
