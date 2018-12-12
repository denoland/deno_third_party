# rust-number-prefix [![number_prefix on crates.io](http://meritbadge.herokuapp.com/number_prefix)](https://crates.io/crates/number_prefix) [![Build status](https://travis-ci.org/ogham/rust-number-prefix.svg?branch=master)](https://travis-ci.org/ogham/rust-number-prefix)

This is a library for formatting numbers with numeric prefixes, such as turning “3000 metres” into “3 kilometres”, or “8705 bytes” into “8.5 KiB”.

### [View the Rustdoc](https://docs.rs/number_prefix)


# Installation

This crate works with [Cargo](http://crates.io). Add the following to your `Cargo.toml` dependencies section:

```toml
[dependencies]
number_prefix = "0.2"
```


# Usage

The function `decimal_prefix` returns either a pair of the resulting number and its prefix, or a notice that the number was too small to have any prefix applied to it.
For example:

```rust
use number_prefix::{decimal_prefix, Standalone, Prefixed};
match decimal_prefix(8542_f32) {
    Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
    Prefixed(prefix, n) => println!("The file is {:.0} {}B in size", n, prefix),
}
```

This will print out `"The file is 8.5 kB in size"`.
The `{:.0}` part of the formatting string tells it to restrict the output to only one decimal place.
This value is calculated by repeatedly dividing the number by 1000 until it becomes less than that, which in this case results in 8.542, which gets rounded down.
Because only one division had to take place, the function also returns the decimal prefix `Kilo`, which gets converted to its internationally-recognised symbol when formatted as a string.

If the value is too small to have any prefixes applied to it - in this case, if it’s under 1000 - then the standalone value will be returned:

```rust
use number_prefix::{decimal_prefix, Standalone, Prefixed};
match decimal_prefix(705_f32) {
    Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
    Prefixed(prefix, n) => println!("The file is {:.0} {}B in size", n, prefix),
}
```

This will print out `"The file is 705 bytes in size"`, having chosen the other path to follow.
In this particular example, the user expects different formatting for both bytes and kilobytes: while prefixed values are given more precision, there’s no point using anything other than whole numbers for just byte amounts.
This is why the function pays attention to values without any prefixes - they often need to be special-cased.


## Binary Prefixes

This library also allows you to use the *binary prefixes*, which use the number 1024 (2^10) as the multiplier, rather than the more common 1000 (10^3).
This uses the `binary_prefix` function. For example:

```rust
use number_prefix::{binary_prefix, Standalone, Prefixed};
match binary_prefix(8542_f32) {
    Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
    Prefixed(prefix, n) => println!("The file is {:.0} {}B in size", n, prefix),
}
```

This will print out `"The file is 8.3 KiB in size"`.
A kibibyte is slightly larger than a kilobyte, so the number is smaller in the result; but other than that, it works in exactly the same way, with the binary prefix being converted to a symbol automatically.


### Which type of prefix should I use?

There is no one way to answer this question!
The prevailing theory is to use the binary prefixes for numbers of *bytes*, while still using the decimal prefixes for everything else.
Computers work with powers of two, rather than powers of ten, and by using the binary prefixes, you get a more accurate representation about the amount of data.


## Prefix Names

If you need to describe your unit in actual words, rather than just with
the symbol, import the `PrefixNames` trait, which adds methods to output
the prefix in a variety of formats. For example:

```rust
use number_prefix::{decimal_prefix, Standalone, Prefixed, PrefixNames};
match decimal_prefix(8542_f32) {
    Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
    Prefixed(prefix, n) => println!("The file is {:.0} {}bytes in size", n, prefix.lower()),
}
```
