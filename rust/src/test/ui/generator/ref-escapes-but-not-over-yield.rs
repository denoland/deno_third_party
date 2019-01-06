#![feature(generators, generator_trait)]

use std::ops::{GeneratorState, Generator};
use std::cell::Cell;

fn foo(x: &i32) {
    // In this case, a reference to `b` escapes the generator, but not
    // because of a yield. We see that there is no yield in the scope of
    // `b` and give the more generic error message.
    let mut a = &3;
    let mut b = move || {
        yield();
        let b = 5;
        a = &b;
        //~^ ERROR `b` does not live long enough
    };
}

fn main() { }
