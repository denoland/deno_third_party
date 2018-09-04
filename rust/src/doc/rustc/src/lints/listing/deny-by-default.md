# Deny-by-default lints

These lints are all set to the 'deny' level by default.

## exceeding-bitshifts

This lint detects that a shift exceeds the type's number of bits. Some
example code that triggers this lint:

```rust,ignore
1_i32 << 32;
```

This will produce:

```text
error: bitshift exceeds the type's number of bits
 --> src/main.rs:2:5
  |
2 |     1_i32 << 32;
  |     ^^^^^^^^^^^
  |
```

## invalid-type-param-default

This lint detects type parameter default erroneously allowed in invalid location. Some
example code that triggers this lint:

```rust,ignore
fn foo<T=i32>(t: T) {}
```

This will produce:

```text
error: defaults for type parameters are only allowed in `struct`, `enum`, `type`, or `trait` definitions.
 --> src/main.rs:4:8
  |
4 | fn foo<T=i32>(t: T) {}
  |        ^
  |
  = note: #[deny(invalid_type_param_default)] on by default
  = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
  = note: for more information, see issue #36887 <https://github.com/rust-lang/rust/issues/36887>
```

## legacy-constructor-visibility

[RFC 1506](https://github.com/rust-lang/rfcs/blob/master/text/1506-adt-kinds.md) modified some
visibility rules, and changed the visibility of struct constructors. Some
example code that triggers this lint:

```rust,ignore
mod m {
    pub struct S(u8);
    
    fn f() {
        // this is trying to use S from the 'use' line, but becuase the `u8` is
        // not pub, it is private
        ::S;
    }
}

use m::S;
```

This will produce:

```text
error: private struct constructors are not usable through re-exports in outer modules
 --> src/main.rs:5:9
  |
5 |         ::S;
  |         ^^^
  |
  = note: #[deny(legacy_constructor_visibility)] on by default
  = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
  = note: for more information, see issue #39207 <https://github.com/rust-lang/rust/issues/39207>
```


## legacy-directory-ownership

The legacy_directory_ownership warning is issued when

* There is a non-inline module with a #[path] attribute (e.g. #[path = "foo.rs"] mod bar;),
* The module's file ("foo.rs" in the above example) is not named "mod.rs", and
* The module's file contains a non-inline child module without a #[path] attribute.

The warning can be fixed by renaming the parent module to "mod.rs" and moving
it into its own directory if appropriate.


## missing-fragment-specifier

The missing_fragment_specifier warning is issued when an unused pattern in a
`macro_rules!` macro definition has a meta-variable (e.g. `$e`) that is not
followed by a fragment specifier (e.g. `:expr`).

This warning can always be fixed by removing the unused pattern in the
`macro_rules!` macro definition.

## mutable-transmutes

This lint catches transmuting from `&T` to `&mut T` becuase it is undefined
behavior. Some example code that triggers this lint:

```rust,ignore
unsafe {
    let y = std::mem::transmute::<&i32, &mut i32>(&5);
}
```

This will produce:

```text
error: mutating transmuted &mut T from &T may cause undefined behavior, consider instead using an UnsafeCell
 --> src/main.rs:3:17
  |
3 |         let y = std::mem::transmute::<&i32, &mut i32>(&5);
  |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
```


## no-mangle-const-items

This lint detects any `const` items with the `#[no_mangle]` attribute.
Constants do not have their symbols exported, and therefore, this probably
means you meant to use a `static`, not a `const`. Some example code that
triggers this lint:

```rust,ignore
#[no_mangle]
const FOO: i32 = 5;
```

This will produce:

```text
error: const items should never be #[no_mangle]
 --> src/main.rs:3:1
  |
3 | const FOO: i32 = 5;
  | -----^^^^^^^^^^^^^^
  | |
  | help: try a static value: `pub static`
  |
```

## parenthesized-params-in-types-and-modules

This lint detects incorrect parentheses. Some example code that triggers this
lint:

```rust,ignore
let x = 5 as usize();
```

This will produce:

```text
error: parenthesized parameters may only be used with a trait
 --> src/main.rs:2:21
  |
2 |   let x = 5 as usize();
  |                     ^^
  |
  = note: #[deny(parenthesized_params_in_types_and_modules)] on by default
  = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
  = note: for more information, see issue #42238 <https://github.com/rust-lang/rust/issues/42238>
```

To fix it, remove the `()`s.

## pub-use-of-private-extern-crate

This lint detects a specific situation of re-exporting a private `extern crate`;

## safe-extern-statics

In older versions of Rust, there was a soundness issue where `extern static`s were allowed
to be accessed in safe code. This lint now catches and denies this kind of code.

## unknown-crate-types

This lint detects an unknown crate type found in a `#[crate_type]` directive. Some
example code that triggers this lint:

```rust,ignore
#![crate_type="lol"]
```

This will produce:

```text
error: invalid `crate_type` value
 --> src/lib.rs:1:1
  |
1 | #![crate_type="lol"]
  | ^^^^^^^^^^^^^^^^^^^^
  |
```

## incoherent-fundamental-impls

This lint detects potentially-conflicting impls that were erroneously allowed. Some
example code that triggers this lint:

```rust,ignore
pub trait Trait1<X> {
    type Output;
}

pub trait Trait2<X> {}

pub struct A;

impl<X, T> Trait1<X> for T where T: Trait2<X> {
    type Output = ();
}

impl<X> Trait1<Box<X>> for A {
    type Output = i32;
}
```

This will produce:

```text
error: conflicting implementations of trait `Trait1<std::boxed::Box<_>>` for type `A`: (E0119)
  --> src/main.rs:13:1
   |
9  | impl<X, T> Trait1<X> for T where T: Trait2<X> {
   | --------------------------------------------- first implementation here
...
13 | impl<X> Trait1<Box<X>> for A {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `A`
   |
   = note: #[deny(incoherent_fundamental_impls)] on by default
   = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
   = note: for more information, see issue #46205 <https://github.com/rust-lang/rust/issues/46205>
   = note: downstream crates may implement trait `Trait2<std::boxed::Box<_>>` for type `A`
```
