// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// revisions: ast mir
//[mir]compile-flags: -Z borrowck=mir

// Ensure that moves out of static items is forbidden

struct Foo {
    foo: isize,
}

static BAR: Foo = Foo { foo: 5 };


fn test(f: Foo) {
    let _f = Foo{foo: 4, ..f};
}

fn main() {
    test(BAR); //[ast]~ ERROR cannot move out of static item [E0507]
               //[mir]~^ ERROR [E0507]
}
