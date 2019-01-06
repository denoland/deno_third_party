#![allow(const_err)]

union Foo {
    a: &'static u32,
    b: usize,
}

fn main() {
    let x: &'static bool = &unsafe { //~ borrowed value does not live long enough
        Foo { a: &1 }.b == Foo { a: &2 }.b
    };
}
