pub fn f(
    /// Comment
    //~^ ERROR documentation comments cannot be applied to method arguments
    //~| NOTE doc comments are not allowed here
    id: u8,
    /// Other
    //~^ ERROR documentation comments cannot be applied to method arguments
    //~| NOTE doc comments are not allowed here
    a: u8,
) {}

fn foo(#[allow(dead_code)] id: i32) {}
//~^ ERROR attributes cannot be applied to method arguments
//~| NOTE attributes are not allowed here

fn bar(id: #[allow(dead_code)] i32) {}
//~^ ERROR attributes cannot be applied to a method argument's type
//~| NOTE attributes are not allowed here

fn main() {
    // verify that the parser recovered and properly typechecked the args
    f("", "");
    //~^ ERROR mismatched types
    //~| NOTE expected u8, found reference
    //~| NOTE expected
    //~| ERROR mismatched types
    //~| NOTE expected u8, found reference
    //~| NOTE expected
    foo("");
    //~^ ERROR mismatched types
    //~| NOTE expected i32, found reference
    //~| NOTE expected
    bar("");
    //~^ ERROR mismatched types
    //~| NOTE expected i32, found reference
    //~| NOTE expected
}
