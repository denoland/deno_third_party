// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py match'

fn main() {
    let match = "foo"; //~ error: expected pattern, found keyword `match`
}
