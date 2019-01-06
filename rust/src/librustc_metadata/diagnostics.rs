#![allow(non_snake_case)]

register_long_diagnostics! {
E0454: r##"
A link name was given with an empty name. Erroneous code example:

```ignore (cannot-test-this-because-rustdoc-stops-compile-fail-before-codegen)
#[link(name = "")] extern {} // error: #[link(name = "")] given with empty name
```

The rust compiler cannot link to an external library if you don't give it its
name. Example:

```no_run
#[link(name = "some_lib")] extern {} // ok!
```
"##,

E0455: r##"
Linking with `kind=framework` is only supported when targeting macOS,
as frameworks are specific to that operating system.

Erroneous code example:

```ignore (should-compile_fail-but-cannot-doctest-conditionally-without-macos)
#[link(name = "FooCoreServices", kind = "framework")] extern {}
// OS used to compile is Linux for example
```

To solve this error you can use conditional compilation:

```
#[cfg_attr(target="macos", link(name = "FooCoreServices", kind = "framework"))]
extern {}
```

See more:
https://doc.rust-lang.org/book/first-edition/conditional-compilation.html
"##,

E0458: r##"
An unknown "kind" was specified for a link attribute. Erroneous code example:

```ignore (cannot-test-this-because-rustdoc-stops-compile-fail-before-codegen)
#[link(kind = "wonderful_unicorn")] extern {}
// error: unknown kind: `wonderful_unicorn`
```

Please specify a valid "kind" value, from one of the following:

* static
* dylib
* framework

"##,

E0459: r##"
A link was used without a name parameter. Erroneous code example:

```ignore (cannot-test-this-because-rustdoc-stops-compile-fail-before-codegen)
#[link(kind = "dylib")] extern {}
// error: #[link(...)] specified without `name = "foo"`
```

Please add the name parameter to allow the rust compiler to find the library
you want. Example:

```no_run
#[link(kind = "dylib", name = "some_lib")] extern {} // ok!
```
"##,

E0463: r##"
A plugin/crate was declared but cannot be found. Erroneous code example:

```compile_fail,E0463
#![feature(plugin)]
#![plugin(cookie_monster)] // error: can't find crate for `cookie_monster`
extern crate cake_is_a_lie; // error: can't find crate for `cake_is_a_lie`
```

You need to link your code to the relevant crate in order to be able to use it
(through Cargo or the `-L` option of rustc example). Plugins are crates as
well, and you link to them the same way.
"##,

}

register_diagnostics! {
    E0456, // plugin `..` is not available for triple `..`
    E0457, // plugin `..` only found in rlib format, but must be available...
    E0514, // metadata version mismatch
    E0460, // found possibly newer version of crate `..`
    E0461, // couldn't find crate `..` with expected target triple ..
    E0462, // found staticlib `..` instead of rlib or dylib
    E0464, // multiple matching crates for `..`
    E0465, // multiple .. candidates for `..` found
    E0519, // local crate and dependency have same (crate-name, disambiguator)
    E0523, // two dependencies have same (crate-name, disambiguator) but different SVH
}
