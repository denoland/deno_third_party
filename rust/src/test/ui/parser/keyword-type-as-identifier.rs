// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py type'

fn main() {
    let type = "foo"; //~ error: expected pattern, found keyword `type`
}
