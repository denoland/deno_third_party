// compile-flags: -Z parse-only -Z continue-parse-after-error

struct X {
    a: u8 /// document
    //~^ ERROR found a documentation comment that doesn't document anything
    //~| HELP maybe a comment was intended
}

fn main() {
    let y = X {a: 1};
}
