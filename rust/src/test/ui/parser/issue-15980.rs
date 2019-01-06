// compile-flags: -Z parse-only

use std::io;

fn main(){
    let x: io::IoResult<()> = Ok(());
    match x {
        Err(ref e) if e.kind == io::EndOfFile {
            //~^ NOTE while parsing this struct
            return
            //~^ ERROR expected identifier, found keyword `return`
            //~| NOTE expected identifier, found keyword
        }
        //~^ NOTE expected one of `.`, `=>`, `?`, or an operator here
        _ => {}
        //~^ ERROR expected one of `.`, `=>`, `?`, or an operator, found `_`
        //~| NOTE unexpected token
    }
}
