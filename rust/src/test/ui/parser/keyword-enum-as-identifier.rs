// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py enum'

fn main() {
    let enum = "foo"; //~ error: expected pattern, found keyword `enum`
}
