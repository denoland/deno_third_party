// compile-flags: -Z parse-only

fn main() {
    let final = (); //~ ERROR expected pattern, found reserved keyword `final`
}
