// Test that we give a note when the old LUB/GLB algorithm would have
// succeeded but the new code (which is stricter) gives an error.

trait Foo<T, U> { }

fn foo(
    x: &for<'a, 'b> Foo<&'a u8, &'b u8>,
    y: &for<'a> Foo<&'a u8, &'a u8>,
) {
    let z = match 22 { //~ ERROR E0308
        0 => x,
        _ => y,
    };
}

fn bar(
    x: &for<'a, 'b> Foo<&'a u8, &'b u8>,
    y: &for<'a> Foo<&'a u8, &'a u8>,
) {
    // Accepted with explicit case:
    let z = match 22 {
        0 => x as &for<'a> Foo<&'a u8, &'a u8>,
        _ => y,
    };
}

fn main() {
}
