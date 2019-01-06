// Check that existential types must be ungated to use the `existential` keyword



existential type Foo: std::fmt::Debug; //~ ERROR existential types are unstable

trait Bar {
    type Baa: std::fmt::Debug;
}

impl Bar for () {
    existential type Baa: std::fmt::Debug; //~ ERROR existential types are unstable
}

fn main() {}
