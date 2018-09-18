// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::mem::size_of;

#[derive(PartialEq, Debug)]
enum Either<T, U> { Left(T), Right(U) }

macro_rules! check {
    ($t:ty, $sz:expr, $($e:expr, $s:expr),*) => {{
        assert_eq!(size_of::<$t>(), $sz);
        $({
            static S: $t = $e;
            let v: $t = $e;
            assert_eq!(S, v);
            assert_eq!(format!("{:?}", v), $s);
            assert_eq!(format!("{:?}", S), $s);
        });*
    }}
}

pub fn main() {
    check!(Option<u8>, 2,
           None, "None",
           Some(129), "Some(129)");
    check!(Option<i16>, 4,
           None, "None",
           Some(-20000), "Some(-20000)");
    check!(Either<u8, i8>, 2,
           Either::Left(132), "Left(132)",
           Either::Right(-32), "Right(-32)");
    check!(Either<u8, i16>, 4,
           Either::Left(132), "Left(132)",
           Either::Right(-20000), "Right(-20000)");
}
