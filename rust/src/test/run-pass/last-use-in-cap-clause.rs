#![allow(dead_code)]
// Make sure #1399 stays fixed

struct A { a: Box<isize> }

fn foo() -> Box<FnMut() -> isize + 'static> {
    let k: Box<_> = Box::new(22);
    let _u = A {a: k.clone()};
    let result  = || 22;
    Box::new(result)
}

pub fn main() {
    assert_eq!(foo()(), 22);
}
