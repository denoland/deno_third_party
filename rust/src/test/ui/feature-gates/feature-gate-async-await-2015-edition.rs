// edition:2015

#![feature(futures_api)]

async fn foo() {} //~ ERROR async fn is unstable

fn main() {
    let _ = async {}; //~ ERROR cannot find struct, variant or union type `async`
    let _ = async || {}; //~ ERROR cannot find value `async` in this scope
}
