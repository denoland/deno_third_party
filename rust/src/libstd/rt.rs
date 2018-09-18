// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Runtime services
//!
//! The `rt` module provides a narrow set of runtime services,
//! including the global heap (exported in `heap`) and unwinding and
//! backtrace support. The APIs in this module are highly unstable,
//! and should be considered as private implementation details for the
//! time being.

#![unstable(feature = "rt",
            reason = "this public module should not exist and is highly likely \
                      to disappear",
            issue = "0")]
#![doc(hidden)]


// Re-export some of our utilities which are expected by other crates.
pub use panicking::{begin_panic, begin_panic_fmt, update_panic_count};

// To reduce the generated code of the new `lang_start`, this function is doing
// the real work.
#[cfg(not(test))]
fn lang_start_internal(main: &(Fn() -> i32 + Sync + ::panic::RefUnwindSafe),
                       argc: isize, argv: *const *const u8) -> isize {
    use panic;
    use sys;
    use sys_common;
    use sys_common::thread_info;
    use thread::Thread;

    sys::init();

    unsafe {
        let main_guard = sys::thread::guard::init();
        sys::stack_overflow::init();

        // Next, set up the current Thread with the guard information we just
        // created. Note that this isn't necessary in general for new threads,
        // but we just do this to name the main thread and to give it correct
        // info about the stack bounds.
        let thread = Thread::new(Some("main".to_owned()));
        thread_info::set(main_guard, thread);

        // Store our args if necessary in a squirreled away location
        sys::args::init(argc, argv);

        // Let's run some code!
        #[cfg(feature = "backtrace")]
        let exit_code = panic::catch_unwind(|| {
            ::sys_common::backtrace::__rust_begin_short_backtrace(move || main())
        });
        #[cfg(not(feature = "backtrace"))]
        let exit_code = panic::catch_unwind(move || main());

        sys_common::cleanup();
        exit_code.unwrap_or(101) as isize
    }
}

#[cfg(not(test))]
#[lang = "start"]
fn lang_start<T: ::process::Termination + 'static>
    (main: fn() -> T, argc: isize, argv: *const *const u8) -> isize
{
    lang_start_internal(&move || main().report(), argc, argv)
}

/// Function used for reverting changes to the main stack before setrlimit().
/// This is POSIX (non-Linux) specific and unlikely to be directly stabilized.
#[unstable(feature = "rustc_stack_internals", issue = "0")]
pub unsafe fn deinit_stack_guard() {
    ::sys::thread::guard::deinit();
}

/// Function used for resetting the main stack guard address after setrlimit().
/// This is POSIX specific and unlikely to be directly stabilized.
#[unstable(feature = "rustc_stack_internals", issue = "0")]
pub unsafe fn update_stack_guard() {
    let main_guard = ::sys::thread::guard::init();
    ::sys_common::thread_info::reset_guard(main_guard);
}
