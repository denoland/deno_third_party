// compile-flags: -Z parse-only

// issue #17123

fn main() {
    9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999
    //~^ ERROR int literal is too large
        ; // the span shouldn't point to this.
}
