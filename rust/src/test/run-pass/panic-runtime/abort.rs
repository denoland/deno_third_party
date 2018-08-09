// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// compile-flags:-C panic=abort
// no-prefer-dynamic
// ignore-cloudabi no processes
// ignore-emscripten no processes

use std::process::Command;
use std::env;

struct Bomb;

impl Drop for Bomb {
    fn drop(&mut self) {
        std::process::exit(0);
    }
}

fn main() {
    let mut args = env::args_os();
    let me = args.next().unwrap();

    if let Some(s) = args.next() {
        if &*s == "foo" {

            let _bomb = Bomb;

            panic!("try to catch me");
        }
    }

    let mut cmd = Command::new(env::args_os().next().unwrap());
    cmd.arg("foo");

    // ARMv6 hanges while printing the backtrace, see #41004
    if cfg!(target_arch = "arm") && cfg!(target_env = "gnu") {
        cmd.env("RUST_BACKTRACE", "0");
    }

    let s = cmd.status();
    assert!(s.unwrap().code() != Some(0));
}
