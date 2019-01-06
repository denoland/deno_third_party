trait Foo {
    fn bar(&mut self, other: &mut Foo);
}

struct Baz;

impl Foo for Baz {
    fn bar(&mut self, other: &Foo) {}
    //~^ ERROR method `bar` has an incompatible type for trait
    //~| expected type `fn(&mut Baz, &mut dyn Foo)`
    //~| found type `fn(&mut Baz, &dyn Foo)`
}

fn main() {}
