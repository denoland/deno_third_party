// skip-codegen
// compile-pass
#![warn(unused)]

type Z = for<'x> Send;
//~^ WARN type alias is never used


fn main() {
}
