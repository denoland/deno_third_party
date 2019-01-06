// compile-flags: -Z parse-only -Z continue-parse-after-error

fn main() {
    enum Test {
        Var1,
        Var2(String),
        Var3 {
            abc: {}, //~ ERROR: expected type, found `{`
        },
    }

    // recover...
    let a = 1;
    enum Test2 {
        Fine,
    }

    enum Test3 {
        StillFine {
            def: i32,
        },
    }

    {
        // fail again
        enum Test4 {
            Nope(i32 {}) //~ ERROR: found `{`
                         //~^ ERROR: found `{`
        }
    }
    // still recover later
    let bad_syntax = _; //~ ERROR: expected expression, found reserved identifier `_`
}
