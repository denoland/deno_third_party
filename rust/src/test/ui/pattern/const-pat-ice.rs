// failure-status: 101

// This is a repro test for an ICE in our pattern handling of constants.

const FOO: &&&u32 = &&&42;

fn main() {
    match unimplemented!() {
        &&&42 => {},
        FOO => {},
        _ => {},
    }
}
