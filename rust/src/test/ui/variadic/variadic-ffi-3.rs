extern {
    fn foo(f: isize, x: u8, ...);
    //~^ defined here
    //~| defined here
}

extern "C" fn bar(f: isize, x: u8) {}

fn main() {
    unsafe {
        foo(); //~ ERROR: this function takes at least 2 parameters but 0 parameters were supplied
        foo(1); //~ ERROR: this function takes at least 2 parameters but 1 parameter was supplied

        let x: unsafe extern "C" fn(f: isize, x: u8) = foo;
        //~^ ERROR: mismatched types
        //~| expected type `unsafe extern "C" fn(isize, u8)`
        //~| found type `unsafe extern "C" fn(isize, u8, ...) {foo}`

        let y: extern "C" fn(f: isize, x: u8, ...) = bar;
        //~^ ERROR: mismatched types
        //~| expected type `extern "C" fn(isize, u8, ...)`
        //~| found type `extern "C" fn(isize, u8) {bar}`

        foo(1, 2, 3f32); //~ ERROR can't pass `f32` to variadic function
        foo(1, 2, true); //~ ERROR can't pass `bool` to variadic function
        foo(1, 2, 1i8); //~ ERROR can't pass `i8` to variadic function
        foo(1, 2, 1u8); //~ ERROR can't pass `u8` to variadic function
        foo(1, 2, 1i16); //~ ERROR can't pass `i16` to variadic function
        foo(1, 2, 1u16); //~ ERROR can't pass `u16` to variadic function
    }
}
