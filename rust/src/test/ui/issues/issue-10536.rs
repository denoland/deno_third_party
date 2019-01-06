// We only want to assert that this doesn't ICE, we don't particularly care
// about whether it nor it fails to compile.

macro_rules! foo{
    () => {{
        macro_rules! bar{() => (())}
        1
    }}
}

pub fn main() {
    foo!();

    assert!({one! two()});
    //~^ ERROR macros that expand to items must either be surrounded with braces or followed by a
    //~| ERROR cannot find macro `one!` in this scope
    //~| ERROR mismatched types

    // regardless of whether nested macro_rules works, the following should at
    // least throw a conventional error.
    assert!({one! two});
    //~^ ERROR expected `(` or `{`, found `}`
}
