// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use foo::MyEnum::Result;
use foo::NoResult; // Through a re-export

mod foo {
    pub use self::MyEnum::NoResult;

    pub enum MyEnum {
        Result,
        NoResult
    }

    fn new() -> NoResult<MyEnum, String> {
        //~^ ERROR expected type, found variant `NoResult`
        unimplemented!()
    }
}

mod bar {
    use foo::MyEnum::Result;
    use foo;

    fn new() -> Result<foo::MyEnum, String> {
        //~^ ERROR expected type, found variant `Result`
        unimplemented!()
    }
}

fn new() -> Result<foo::MyEnum, String> {
    //~^ ERROR expected type, found variant `Result`
    unimplemented!()
}

fn newer() -> NoResult<foo::MyEnum, String> {
    //~^ ERROR expected type, found variant `NoResult`
    unimplemented!()
}

fn main() {
    let _ = new();
}
