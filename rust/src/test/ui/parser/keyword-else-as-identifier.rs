// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py else'

fn main() {
    let else = "foo"; //~ error: expected pattern, found keyword `else`
}
