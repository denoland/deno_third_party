// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py break'

fn main() {
    let break = "foo"; //~ error: expected pattern, found keyword `break`
}
