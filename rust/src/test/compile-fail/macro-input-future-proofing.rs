// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unused_macros)]

macro_rules! errors_everywhere {
    ($ty:ty <) => (); //~ ERROR `$ty:ty` is followed by `<`, which is not allowed for `ty`
    ($ty:ty < foo ,) => (); //~ ERROR `$ty:ty` is followed by `<`, which is not allowed for `ty`
    ($ty:ty , ) => ();
    ( ( $ty:ty ) ) => ();
    ( { $ty:ty } ) => ();
    ( [ $ty:ty ] ) => ();
    ($bl:block < ) => ();
    ($pa:pat >) => (); //~ ERROR `$pa:pat` is followed by `>`, which is not allowed for `pat`
    ($pa:pat , ) => ();
    ($pa:pat $pb:pat $ty:ty ,) => ();
    //~^ ERROR `$pa:pat` is followed by `$pb:pat`, which is not allowed
    //~^^ ERROR `$pb:pat` is followed by `$ty:ty`, which is not allowed
    ($($ty:ty)* -) => (); //~ ERROR `$ty:ty` is followed by `-`
    ($($a:ty, $b:ty)* -) => (); //~ ERROR `$b:ty` is followed by `-`
    ($($ty:ty)-+) => (); //~ ERROR `$ty:ty` is followed by `-`, which is not allowed for `ty`
    ( $($a:expr)* $($b:tt)* ) => { };
    //~^ ERROR `$a:expr` is followed by `$b:tt`, which is not allowed for `expr` fragments
}

fn main() { }
