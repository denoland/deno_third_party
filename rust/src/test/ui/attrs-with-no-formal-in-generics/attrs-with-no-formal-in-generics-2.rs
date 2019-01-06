// This test checks variations on `<#[attr] 'a, #[oops]>`, where
// `#[oops]` is left dangling (that is, it is unattached, with no
// formal binding following it).

#![feature(rustc_attrs)]

struct RefAny<'a, T>(&'a T);

impl<#[rustc_1] 'a, #[rustc_2] T, #[oops]> RefAny<'a, T> {}
//~^ ERROR trailing attribute after type parameters

fn main() {}
