// compile-flags: -Z parse-only

// This file was auto-generated using 'src/etc/generate-keyword-tests.py where'

fn main() {
    let where = "foo"; //~ error: expected pattern, found keyword `where`
}
