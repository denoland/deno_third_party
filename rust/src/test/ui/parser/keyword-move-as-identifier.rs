// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py move'

fn main() {
    let move = "foo"; //~ error: expected pattern, found keyword `move`
}
