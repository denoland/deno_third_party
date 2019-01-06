#![feature(existential_type)]

fn main() {}

// two definitions with different types
existential type Foo: std::fmt::Debug;

fn foo() -> Foo {
    ""
}

fn bar() -> Foo { //~ ERROR defining existential type use differs from previous
    panic!()
}

fn boo() -> Foo { //~ ERROR defining existential type use differs from previous
    loop {}
}
