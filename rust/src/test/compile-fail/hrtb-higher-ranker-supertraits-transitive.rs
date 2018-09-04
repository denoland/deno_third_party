// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Test HRTB supertraits with several levels of expansion required.

trait Foo<'tcx>
{
    fn foo(&'tcx self) -> &'tcx isize;
}

trait Bar<'ccx>
    : for<'tcx> Foo<'tcx>
{
    fn bar(&'ccx self) -> &'ccx isize;
}

trait Baz
    : for<'ccx> Bar<'ccx>
{
    fn dummy(&self);
}

trait Qux
    : Bar<'static>
{
    fn dummy(&self);
}

fn want_foo_for_any_tcx<F>(f: &F)
    where F : for<'tcx> Foo<'tcx>
{
}

fn want_bar_for_any_ccx<B>(b: &B)
    where B : for<'ccx> Bar<'ccx>
{
}

fn want_baz<B>(b: &B)
    where B : Baz
{
    want_foo_for_any_tcx(b);
    want_bar_for_any_ccx(b);
}

fn want_qux<B>(b: &B)
    where B : Qux
{
    want_foo_for_any_tcx(b);
    want_bar_for_any_ccx(b); //~ ERROR E0277
}

fn main() {}
