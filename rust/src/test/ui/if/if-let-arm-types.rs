fn main() {
    if let Some(b) = None { //~ ERROR: `if let` arms have incompatible types
        //~^ expected (), found integer
        //~| expected type `()`
        //~| found type `{integer}`
        ()
    } else {
        1
    };
}
