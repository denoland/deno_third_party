// Test that we enforce a `&'static` requirement that is only visible
// after normalization.

#![feature(nll)]
#![ignore(unused)]

trait Foo { type Out; }
impl Foo for () { type Out = &'static u32; }

fn main() {
    let a = 22;
    let b: <() as Foo>::Out = &a; //~ ERROR
}
