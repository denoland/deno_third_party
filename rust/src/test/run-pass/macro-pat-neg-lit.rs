// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

macro_rules! enum_number {
    ($name:ident { $($variant:ident = $value:expr, )* }) => {
        enum $name {
            $($variant = $value,)*
        }

        fn foo(value: i32) -> Option<$name> {
            match value {
                $( $value => Some($name::$variant), )*
                _ => None
            }
        }
    }
}

enum_number!(Change {
    Down = -1,
    None = 0,
    Up = 1,
});

fn main() {
    if let Some(Change::Down) = foo(-1) {} else { panic!() }
}

