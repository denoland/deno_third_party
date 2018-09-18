// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use cell::UnsafeCell;
use sys::c;
use sys::mutex::{self, Mutex};
use sys::os;
use time::Duration;

pub struct Condvar { inner: UnsafeCell<c::CONDITION_VARIABLE> }

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

impl Condvar {
    pub const fn new() -> Condvar {
        Condvar { inner: UnsafeCell::new(c::CONDITION_VARIABLE_INIT) }
    }

    #[inline]
    pub unsafe fn init(&mut self) {}

    #[inline]
    pub unsafe fn wait(&self, mutex: &Mutex) {
        let r = c::SleepConditionVariableSRW(self.inner.get(),
                                             mutex::raw(mutex),
                                             c::INFINITE,
                                             0);
        debug_assert!(r != 0);
    }

    pub unsafe fn wait_timeout(&self, mutex: &Mutex, dur: Duration) -> bool {
        let r = c::SleepConditionVariableSRW(self.inner.get(),
                                             mutex::raw(mutex),
                                             super::dur2timeout(dur),
                                             0);
        if r == 0 {
            debug_assert_eq!(os::errno() as usize, c::ERROR_TIMEOUT as usize);
            false
        } else {
            true
        }
    }

    #[inline]
    pub unsafe fn notify_one(&self) {
        c::WakeConditionVariable(self.inner.get())
    }

    #[inline]
    pub unsafe fn notify_all(&self) {
        c::WakeAllConditionVariable(self.inner.get())
    }

    pub unsafe fn destroy(&self) {
        // ...
    }
}
