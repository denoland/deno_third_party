#![crate_name = "number_prefix"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]

#![deny(unsafe_code)]

#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]


//! This is a library for formatting numbers with numeric prefixes, such as
//! turning “3000 metres” into “3 kilometres”, or “8705 bytes” into “8.5 KiB”.
//!
//!
//! # Usage
//!
//! The function `decimal_prefix` returns either a pair of the resulting number
//! and its prefix, or a notice that the number was too small to have any prefix
//! applied to it. For example:
//!
//! ```rust
//! use number_prefix::{decimal_prefix, Standalone, Prefixed};
//! match decimal_prefix(8542_f32) {
//!     Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
//!     Prefixed(prefix, n) => println!("The file is {:.0} {}B in size", n, prefix),
//! }
//! ```
//!
//! This will print out `"The file is 8.5 kB in size"`. The `{:.0}` part of the
//! formatting string tells it to restrict the output to only one decimal place.
//! This value is calculated by repeatedly dividing the number by 1000 until it
//! becomes less than that, which in this case results in 8.542, which gets
//! rounded down. Because only one division had to take place, the function also
//! returns the decimal prefix `Kilo`, which gets converted to its
//! internationally-recognised symbol when formatted as a string.
//!
//! If the value is too small to have any prefixes applied to it - in this case,
//! if it’s under 1000 - then the standalone value will be returned:
//!
//! ```rust
//! use number_prefix::{decimal_prefix, Standalone, Prefixed};
//! match decimal_prefix(705_f32) {
//!     Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
//!     Prefixed(prefix, n) => println!("The file is {:.0} {}B in size", n, prefix),
//! }
//! ```
//!
//! This will print out `"The file is 705 bytes in size"`, having chosen the
//! other path to follow. In this particular example, the user expects different
//! formatting for both bytes and kilobytes: while prefixed values are given
//! more precision, there’s no point using anything other than whole numbers for
//! just byte amounts. This is why the function pays attention to values without
//! any prefixes - they often need to be special-cased.
//!
//!
//! ## Binary Prefixes
//!
//! This library also allows you to use the *binary prefixes*, which use the
//! number 1024 (2^10) as the multiplier, rather than the more common 1000
//! (10^3). This uses the `binary_prefix` function. For example:
//!
//! ```rust
//! use number_prefix::{binary_prefix, Standalone, Prefixed};
//! match binary_prefix(8542_f32) {
//!     Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
//!     Prefixed(prefix, n) => println!("The file is {:.0} {}B in size", n, prefix),
//! }
//! ```
//!
//! This will print out `"The file is 8.3 KiB in size"`. A kibibyte is slightly
//! larger than a kilobyte, so the number is smaller in the result; but other
//! than that, it works in exactly the same way, with the binary prefix being
//! converted to a symbol automatically.
//!
//!
//! ### Which type of prefix should I use?
//!
//! There is no one way to answer this question! The prevailing theory is to use
//! the binary prefixes for numbers of *bytes*, while still using the decimal
//! prefixes for everything else. Computers work with powers of two, rather than
//! powers of ten, and by using the binary prefixes, you get a more accurate
//! representation about the amount of data.
//!
//!
//! ## Prefix Names
//!
//! If you need to describe your unit in actual words, rather than just with the
//! symbol, import the `PrefixNames` trait, which adds methods to output the
//! prefix in a variety of formats. For example:
//!
//! ```rust
//! use number_prefix::{decimal_prefix, Standalone, Prefixed, PrefixNames};
//! match decimal_prefix(8542_f32) {
//!     Standalone(bytes)   => println!("The file is {} bytes in size", bytes),
//!     Prefixed(prefix, n) => println!("The file is {:.0} {}bytes in size", n, prefix.lower()),
//! }
//! ```


extern crate num_traits;
use num_traits::{Float, Signed};
use std::fmt;

pub use Prefix::{
	Kilo, Mega, Giga, Tera, Peta, Exa, Zetta, Yotta,
	Kibi, Mibi, Gibi, Tebi, Pebi, Exbi, Zebi, Yobi,
};

pub use Result::{Standalone, Prefixed};


/// Formatting methods for prefix, for when you want to output things other
/// than just the short-hand symbols.
pub trait PrefixNames {

	/// Returns the name in uppercase, such as “KILO”.
    fn upper(&self) -> &'static str;

    /// Returns the name with the first letter capitalised, such as “Mega”.
    fn caps(&self) -> &'static str;

    /// Returns the name in lowercase, such as “giga”.
    fn lower(&self) -> &'static str;

    /// Returns the short-hand symbol, such as “T” (for “tera”).
    fn symbol(&self) -> &'static str;
}

/// A numeric prefix, either binary or decimal.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Prefix {
    Kilo, Mega, Giga, Tera, Peta, Exa, Zetta, Yotta,
    Kibi, Mibi, Gibi, Tebi, Pebi, Exbi, Zebi, Yobi,
}

/// Formats the given floating-point number using **decimal** prefixes,
/// returning a result.
///
/// This function accepts both `f32` and `f64` values. If you’re trying to
/// format an integer, you’ll have to cast it first.
pub fn decimal_prefix<F: Amounts>(amount: F) -> Result<F> {
	format_number(amount, Amounts::get_1000(), [Kilo, Mega, Giga, Tera, Peta, Exa, Zetta, Yotta])
}

/// Formats the given floating-point number using **binary** prefixes,
/// returning a result.
///
/// This function accepts both `f32` and `f64` values. If you’re trying to
/// format an integer, you’ll have to cast it first.
pub fn binary_prefix<F: Amounts>(amount: F) -> Result<F> {
	format_number(amount, Amounts::get_1024(), [Kibi, Mibi, Gibi, Tebi, Pebi, Exbi, Zebi, Yobi])
}

impl fmt::Display for Prefix {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.symbol())
	}
}

impl PrefixNames for Prefix {
    fn upper(&self) -> &'static str {
        match *self {
            Kilo => "KILO",  Mega => "MEGA",  Giga  => "GIGA",   Tera  => "TERA",
            Peta => "PETA",  Exa  => "EXA",   Zetta => "ZETTA",  Yotta => "YOTTA",
            Kibi => "KIBI",  Mibi => "MIBI",  Gibi  => "GIBI",   Tebi  => "TEBI",
            Pebi => "PEBI",  Exbi => "EXBI",  Zebi  => "ZEBI",   Yobi  => "YOBI",
        }
    }

    fn caps(&self) -> &'static str {
        match *self {
            Kilo => "Kilo",  Mega => "Mega",  Giga  => "Giga",   Tera  => "Tera",
            Peta => "Peta",  Exa  => "Exa",   Zetta => "Zetta",  Yotta => "Yotta",
            Kibi => "Kibi",  Mibi => "Mibi",  Gibi  => "Gibi",   Tebi  => "Tebi",
            Pebi => "Pebi",  Exbi => "Exbi",  Zebi  => "Zebi",   Yobi  => "Yobi",
        }
    }

    fn lower(&self) -> &'static str {
        match *self {
            Kilo => "kilo",  Mega => "mega",  Giga  => "giga",   Tera  => "tera",
            Peta => "peta",  Exa  => "exa",   Zetta => "zetta",  Yotta => "yotta",
            Kibi => "kibi",  Mibi => "mibi",  Gibi  => "gibi",   Tebi  => "tebi",
            Pebi => "pebi",  Exbi => "exbi",  Zebi  => "zebi",   Yobi  => "yobi",
        }
    }

    fn symbol(&self) -> &'static str {
        match *self {
            Kilo => "k",   Mega => "M",   Giga  => "G",   Tera  => "T",
            Peta => "P",   Exa  => "E",   Zetta => "Z",   Yotta => "Y",
            Kibi => "Ki",  Mibi => "Mi",  Gibi  => "Gi",  Tebi  => "Ti",
            Pebi => "Pi",  Exbi => "Ei",  Zebi  => "Zi",  Yobi  => "Yi",
        }
    }
}

/// The result of trying to apply a prefix to a floating-point value.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Result<F> {

	/// A **standalone** value is returned when the number is too small to
	/// have any prefixes applied to it. This is commonly a special case, so
	/// is handled separately.
    Standalone(F),

    /// A **prefixed** value *is* large enough for prefixes. This holds the
    /// prefix, as well as the resulting value.
    Prefixed(Prefix, F),
}

fn format_number<F>(mut amount: F, kilo: F, prefixes: [Prefix; 8]) -> Result<F>
where F: Float + Signed {

    // For negative numbers, flip it to positive, do the processing, then
    // flip it back to negative again afterwards.
	let was_negative = if amount.is_negative() { amount = -amount; true } else { false };

    let mut prefix = 0;
    while amount >= kilo && prefix < 8 {
        amount = amount / kilo;
        prefix += 1;
    }

    if was_negative {
    	amount = -amount;
    }

    if prefix == 0 {
        Result::Standalone(amount)
    }
    else {
        Result::Prefixed(prefixes[prefix - 1], amount)
    }
}

/// Constructors for floating-point values for both the possible multipliers.
pub trait Amounts: Float+Signed {
    fn get_1000() -> Self;
    fn get_1024() -> Self;
}

impl Amounts for f32 {
    fn get_1000() -> f32 { 1000f32 }
    fn get_1024() -> f32 { 1024f32 }
}

impl Amounts for f64 {
    fn get_1000() -> f64 { 1000f64 }
    fn get_1024() -> f64 { 1024f64 }
}

#[cfg(test)]
mod test {
    use super::{binary_prefix, decimal_prefix};
    use super::Result::{Standalone, Prefixed};
    use super::Prefix::{Kilo, Giga, Tera, Peta, Exa, Zetta, Yotta, Kibi, Mibi, Gibi};

	#[test]
	fn decimal_minus_one_billion() {
	    assert_eq!(decimal_prefix(-1_000_000_000_f64), Prefixed(Giga, -1f64))
	}

    #[test]
    fn decimal_minus_one() {
        assert_eq!(decimal_prefix(-1f64), Standalone(-1f64))
    }

    #[test]
    fn decimal_0() {
        assert_eq!(decimal_prefix(0f64), Standalone(0f64))
    }

    #[test]
    fn decimal_999() {
        assert_eq!(decimal_prefix(999f32), Standalone(999f32))
    }

    #[test]
    fn decimal_1000() {
        assert_eq!(decimal_prefix(1000f32), Prefixed(Kilo, 1f32))
    }

    #[test]
    fn decimal_1030() {
        assert_eq!(decimal_prefix(1030f32), Prefixed(Kilo, 1.03f32))
    }

    #[test]
    fn decimal_1100() {
        assert_eq!(decimal_prefix(1100f64), Prefixed(Kilo, 1.1f64))
    }

    #[test]
    fn decimal_1111() {
        assert_eq!(decimal_prefix(1111f64), Prefixed(Kilo, 1.111f64))
    }

    #[test]
    fn binary_126456() {
        assert_eq!(binary_prefix(126_456f32), Prefixed(Kibi, 123.492188f32))
    }

    #[test]
    fn binary_1048576() {
        assert_eq!(binary_prefix(1_048_576f64), Prefixed(Mibi, 1f64))
    }

    #[test]
    fn binary_1073741824() {
        assert_eq!(binary_prefix(2_147_483_648f32), Prefixed(Gibi, 2f32))
    }

    #[test]
    fn giga() {
    	assert_eq!(decimal_prefix(1_000_000_000f64), Prefixed(Giga, 1f64))
    }

    #[test]
    fn tera() {
    	assert_eq!(decimal_prefix(1_000_000_000_000f64), Prefixed(Tera, 1f64))
    }

    #[test]
    fn peta() {
    	assert_eq!(decimal_prefix(1_000_000_000_000_000f64), Prefixed(Peta, 1f64))
    }

    #[test]
    fn exa() {
    	assert_eq!(decimal_prefix(1_000_000_000_000_000_000f64), Prefixed(Exa, 1f64))
    }

    #[test]
    fn zetta() {
    	assert_eq!(decimal_prefix(1_000_000_000_000_000_000_000f64), Prefixed(Zetta, 1f64))
    }

    #[test]
    fn yotta() {
    	assert_eq!(decimal_prefix(1_000_000_000_000_000_000_000_000f64), Prefixed(Yotta, 1f64))
    }

    #[test]
    #[allow(overflowing_literals)]
    fn and_so_on() {
    	// When you hit yotta, don't keep going
		assert_eq!(decimal_prefix(1_000_000_000_000_000_000_000_000_000f64), Prefixed(Yotta, 1000f64))
    }

    #[test]
    fn example_one() {
		let result = match decimal_prefix(8542_f32) {
			Standalone(bytes)   => format!("The file is {} bytes in size", bytes),
			Prefixed(prefix, n) => format!("The file is {:.1} {}B in size", n, prefix),
		};

		assert_eq!(result, "The file is 8.5 kB in size");
    }

    #[test]
    fn example_two() {
		let result = match decimal_prefix(705_f32) {
			Standalone(bytes)   => format!("The file is {} bytes in size", bytes),
			Prefixed(prefix, n) => format!("The file is {:.1} {}B in size", n, prefix),
		};

		assert_eq!(result, "The file is 705 bytes in size");
    }

	#[test]
    fn example_three() {
		let result = match binary_prefix(8542_f32) {
			Standalone(bytes)   => format!("The file is {} bytes in size", bytes),
			Prefixed(prefix, n) => format!("The file is {:.1} {}B in size", n, prefix),
		};

		assert_eq!(result, "The file is 8.3 KiB in size");
    }
}
