// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use boxed::FnBox;
use io;
use ffi::CStr;
use mem;
use libc::c_void;
use ptr;
use sys::c;
use sys::handle::Handle;
use sys_common::thread::*;
use time::Duration;

use super::to_u16s;

pub const DEFAULT_MIN_STACK_SIZE: usize = 2 * 1024 * 1024;

pub struct Thread {
    handle: Handle
}

impl Thread {
    pub unsafe fn new<'a>(stack: usize, p: Box<FnBox() + 'a>)
                          -> io::Result<Thread> {
        let p = box p;

        // FIXME On UNIX, we guard against stack sizes that are too small but
        // that's because pthreads enforces that stacks are at least
        // PTHREAD_STACK_MIN bytes big.  Windows has no such lower limit, it's
        // just that below a certain threshold you can't do anything useful.
        // That threshold is application and architecture-specific, however.
        // Round up to the next 64 kB because that's what the NT kernel does,
        // might as well make it explicit.
        let stack_size = (stack + 0xfffe) & (!0xfffe);
        let ret = c::CreateThread(ptr::null_mut(), stack_size,
                                  thread_start, &*p as *const _ as *mut _,
                                  0, ptr::null_mut());

        return if ret as usize == 0 {
            Err(io::Error::last_os_error())
        } else {
            mem::forget(p); // ownership passed to CreateThread
            Ok(Thread { handle: Handle::new(ret) })
        };

        extern "system" fn thread_start(main: *mut c_void) -> c::DWORD {
            unsafe { start_thread(main as *mut u8); }
            0
        }
    }

    pub fn set_name(name: &CStr) {
        if let Ok(utf8) = name.to_str() {
            if let Ok(utf16) = to_u16s(utf8) {
                unsafe { c::SetThreadDescription(c::GetCurrentThread(), utf16.as_ptr()); };
            };
        };
    }

    pub fn join(self) {
        let rc = unsafe { c::WaitForSingleObject(self.handle.raw(), c::INFINITE) };
        if rc == c::WAIT_FAILED {
            panic!("failed to join on thread: {}",
                   io::Error::last_os_error());
        }
    }

    pub fn yield_now() {
        // This function will return 0 if there are no other threads to execute,
        // but this also means that the yield was useless so this isn't really a
        // case that needs to be worried about.
        unsafe { c::SwitchToThread(); }
    }

    pub fn sleep(dur: Duration) {
        unsafe {
            c::Sleep(super::dur2timeout(dur))
        }
    }

    pub fn handle(&self) -> &Handle { &self.handle }

    pub fn into_handle(self) -> Handle { self.handle }
}

#[cfg_attr(test, allow(dead_code))]
pub mod guard {
    pub type Guard = !;
    pub unsafe fn current() -> Option<Guard> { None }
    pub unsafe fn init() -> Option<Guard> { None }
    pub unsafe fn deinit() {}
}
