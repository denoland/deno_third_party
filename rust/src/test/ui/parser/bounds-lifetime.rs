// compile-flags: -Z parse-only

type A = for<'a:> fn(); // OK
type A = for<'a:,> fn(); // OK
type A = for<'a> fn(); // OK
type A = for<> fn(); // OK
type A = for<'a: 'b + 'c> fn(); // OK (rejected later by ast_validation)
type A = for<'a: 'b,> fn(); // OK(rejected later by ast_validation)
type A = for<'a: 'b +> fn(); // OK (rejected later by ast_validation)
type A = for<'a, T> fn(); // OK (rejected later by ast_validation)
type A = for<,> fn(); //~ ERROR expected one of `>`, identifier, or lifetime, found `,`

fn main() {}
