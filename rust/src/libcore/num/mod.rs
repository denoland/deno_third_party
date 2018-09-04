// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Numeric traits and functions for the built-in numeric types.

#![stable(feature = "rust1", since = "1.0.0")]

use convert::TryFrom;
use fmt;
use intrinsics;
use mem;
use nonzero::NonZero;
use ops;
use str::FromStr;

macro_rules! impl_nonzero_fmt {
    ( ( $( $Trait: ident ),+ ) for $Ty: ident ) => {
        $(
            #[stable(feature = "nonzero", since = "1.28.0")]
            impl fmt::$Trait for $Ty {
                #[inline]
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    self.get().fmt(f)
                }
            }
        )+
    }
}

macro_rules! nonzero_integers {
    ( $( $Ty: ident($Int: ty); )+ ) => {
        $(
            /// An integer that is known not to equal zero.
            ///
            /// This enables some memory layout optimization.
            /// For example, `Option<NonZeroU32>` is the same size as `u32`:
            ///
            /// ```rust
            /// use std::mem::size_of;
            /// assert_eq!(size_of::<Option<std::num::NonZeroU32>>(), size_of::<u32>());
            /// ```
            #[stable(feature = "nonzero", since = "1.28.0")]
            #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
            pub struct $Ty(NonZero<$Int>);

            impl $Ty {
                /// Create a non-zero without checking the value.
                ///
                /// # Safety
                ///
                /// The value must not be zero.
                #[stable(feature = "nonzero", since = "1.28.0")]
                #[inline]
                pub const unsafe fn new_unchecked(n: $Int) -> Self {
                    $Ty(NonZero(n))
                }

                /// Create a non-zero if the given value is not zero.
                #[stable(feature = "nonzero", since = "1.28.0")]
                #[inline]
                pub fn new(n: $Int) -> Option<Self> {
                    if n != 0 {
                        Some($Ty(NonZero(n)))
                    } else {
                        None
                    }
                }

                /// Returns the value as a primitive type.
                #[stable(feature = "nonzero", since = "1.28.0")]
                #[inline]
                pub fn get(self) -> $Int {
                    self.0 .0
                }

            }

            impl_nonzero_fmt! {
                (Debug, Display, Binary, Octal, LowerHex, UpperHex) for $Ty
            }
        )+
    }
}

nonzero_integers! {
    NonZeroU8(u8);
    NonZeroU16(u16);
    NonZeroU32(u32);
    NonZeroU64(u64);
    NonZeroU128(u128);
    NonZeroUsize(usize);
}

/// Provides intentionally-wrapped arithmetic on `T`.
///
/// Operations like `+` on `u32` values is intended to never overflow,
/// and in some debug configurations overflow is detected and results
/// in a panic. While most arithmetic falls into this category, some
/// code explicitly expects and relies upon modular arithmetic (e.g.,
/// hashing).
///
/// Wrapping arithmetic can be achieved either through methods like
/// `wrapping_add`, or through the `Wrapping<T>` type, which says that
/// all standard arithmetic operations on the underlying value are
/// intended to have wrapping semantics.
///
/// # Examples
///
/// ```
/// use std::num::Wrapping;
///
/// let zero = Wrapping(0u32);
/// let one = Wrapping(1u32);
///
/// assert_eq!(std::u32::MAX, (zero - one).0);
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash)]
pub struct Wrapping<T>(#[stable(feature = "rust1", since = "1.0.0")]
                       pub T);

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: fmt::Debug> fmt::Debug for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[stable(feature = "wrapping_display", since = "1.10.0")]
impl<T: fmt::Display> fmt::Display for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[stable(feature = "wrapping_fmt", since = "1.11.0")]
impl<T: fmt::Binary> fmt::Binary for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[stable(feature = "wrapping_fmt", since = "1.11.0")]
impl<T: fmt::Octal> fmt::Octal for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[stable(feature = "wrapping_fmt", since = "1.11.0")]
impl<T: fmt::LowerHex> fmt::LowerHex for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[stable(feature = "wrapping_fmt", since = "1.11.0")]
impl<T: fmt::UpperHex> fmt::UpperHex for Wrapping<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

// All these modules are technically private and only exposed for coretests:
pub mod flt2dec;
pub mod dec2flt;
pub mod bignum;
pub mod diy_float;

macro_rules! doc_comment {
    ($x:expr, $($tt:tt)*) => {
        #[doc = $x]
        $($tt)*
    };
}

mod wrapping;

// `Int` + `SignedInt` implemented for signed integers
macro_rules! int_impl {
    ($SelfT:ty, $ActualT:ident, $UnsignedT:ty, $BITS:expr, $Min:expr, $Max:expr, $Feature:expr,
     $EndFeature:expr) => {
        doc_comment! {
            concat!("Returns the smallest value that can be represented by this integer type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::min_value(), ", stringify!($Min), ");",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub const fn min_value() -> Self {
                !0 ^ ((!0 as $UnsignedT) >> 1) as Self
            }
        }

        doc_comment! {
            concat!("Returns the largest value that can be represented by this integer type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::max_value(), ", stringify!($Max), ");",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub const fn max_value() -> Self {
                !Self::min_value()
            }
        }

        doc_comment! {
            concat!("Converts a string slice in a given base to an integer.

The string is expected to be an optional `+` or `-` sign followed by digits.
Leading and trailing whitespace represent an error. Digits are a subset of these characters,
depending on `radix`:

 * `0-9`
 * `a-z`
 * `A-Z`

# Panics

This function panics if `radix` is not in the range from 2 to 36.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::from_str_radix(\"A\", 16), Ok(10));",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
                from_str_radix(src, radix)
            }
        }

        doc_comment! {
            concat!("Returns the number of ones in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "let n = 0b100_0000", stringify!($SelfT), ";

assert_eq!(n.count_ones(), 1);",
$EndFeature, "
```
"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn count_ones(self) -> u32 { (self as $UnsignedT).count_ones() }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn count_ones(self) -> u32 { (self as $UnsignedT).count_ones() }
        }

        doc_comment! {
            concat!("Returns the number of zeros in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::max_value().count_zeros(), 1);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn count_zeros(self) -> u32 {
                (!self).count_ones()
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentatio"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn count_zeros(self) -> u32 {
                (!self).count_ones()
            }
        }

        doc_comment! {
            concat!("Returns the number of leading zeros in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "let n = -1", stringify!($SelfT), ";

assert_eq!(n.leading_zeros(), 0);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn leading_zeros(self) -> u32 {
                (self as $UnsignedT).leading_zeros()
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn leading_zeros(self) -> u32 {
                (self as $UnsignedT).leading_zeros()
            }
        }

        doc_comment! {
            concat!("Returns the number of trailing zeros in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "let n = -4", stringify!($SelfT), ";

assert_eq!(n.trailing_zeros(), 2);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn trailing_zeros(self) -> u32 {
                (self as $UnsignedT).trailing_zeros()
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn trailing_zeros(self) -> u32 {
                (self as $UnsignedT).trailing_zeros()
            }
        }

        /// Shifts the bits to the left by a specified amount, `n`,
        /// wrapping the truncated bits to the end of the resulting integer.
        ///
        /// Please note this isn't the same operation as `<<`!
        ///
        /// # Examples
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `i64` is used here.
        ///
        /// Basic usage:
        ///
        /// ```
        /// let n = 0x0123456789ABCDEFi64;
        /// let m = -0x76543210FEDCBA99i64;
        ///
        /// assert_eq!(n.rotate_left(32), m);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[inline]
        pub fn rotate_left(self, n: u32) -> Self {
            (self as $UnsignedT).rotate_left(n) as Self
        }

        /// Shifts the bits to the right by a specified amount, `n`,
        /// wrapping the truncated bits to the beginning of the resulting
        /// integer.
        ///
        /// Please note this isn't the same operation as `>>`!
        ///
        /// # Examples
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `i64` is used here.
        ///
        /// Basic usage:
        ///
        /// ```
        /// let n = 0x0123456789ABCDEFi64;
        /// let m = -0xFEDCBA987654322i64;
        ///
        /// assert_eq!(n.rotate_right(4), m);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[inline]
        pub fn rotate_right(self, n: u32) -> Self {
            (self as $UnsignedT).rotate_right(n) as Self
        }

        /// Reverses the byte order of the integer.
        ///
        /// # Examples
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `i16` is used here.
        ///
        /// Basic usage:
        ///
        /// ```
        /// let n: i16 = 0b0000000_01010101;
        /// assert_eq!(n, 85);
        ///
        /// let m = n.swap_bytes();
        ///
        /// assert_eq!(m, 0b01010101_00000000);
        /// assert_eq!(m, 21760);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[cfg(not(stage0))]
        #[rustc_const_unstable(feature = "const_int_ops")]
        #[inline]
        pub const fn swap_bytes(self) -> Self {
            (self as $UnsignedT).swap_bytes() as Self
        }

        /// Dummy docs. See !stage0 documentation.
        #[stable(feature = "rust1", since = "1.0.0")]
        #[cfg(stage0)]
        #[inline]
        pub fn swap_bytes(self) -> Self {
            (self as $UnsignedT).swap_bytes() as Self
        }

        /// Reverses the bit pattern of the integer.
        ///
        /// # Examples
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `i16` is used here.
        ///
        /// Basic usage:
        ///
        /// ```
        /// #![feature(reverse_bits)]
        ///
        /// let n: i16 = 0b0000000_01010101;
        /// assert_eq!(n, 85);
        ///
        /// let m = n.reverse_bits();
        ///
        /// assert_eq!(m as u16, 0b10101010_00000000);
        /// assert_eq!(m, -22016);
        /// ```
        #[unstable(feature = "reverse_bits", issue = "48763")]
        #[inline]
        pub fn reverse_bits(self) -> Self {
            (self as $UnsignedT).reverse_bits() as Self
        }

        doc_comment! {
            concat!("Converts an integer from big endian to the target's endianness.

On big endian this is a no-op. On little endian the bytes are swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"big\") {
    assert_eq!(", stringify!($SelfT), "::from_be(n), n)
} else {
    assert_eq!(", stringify!($SelfT), "::from_be(n), n.swap_bytes())
}",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn from_be(x: Self) -> Self {
                #[cfg(target_endian = "big")]
                {
                    x
                }
                #[cfg(not(target_endian = "big"))]
                {
                    x.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn from_be(x: Self) -> Self {
                if cfg!(target_endian = "big") { x } else { x.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Converts an integer from little endian to the target's endianness.

On little endian this is a no-op. On big endian the bytes are swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"little\") {
    assert_eq!(", stringify!($SelfT), "::from_le(n), n)
} else {
    assert_eq!(", stringify!($SelfT), "::from_le(n), n.swap_bytes())
}",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn from_le(x: Self) -> Self {
                #[cfg(target_endian = "little")]
                {
                    x
                }
                #[cfg(not(target_endian = "little"))]
                {
                    x.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn from_le(x: Self) -> Self {
                if cfg!(target_endian = "little") { x } else { x.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Converts `self` to big endian from the target's endianness.

On big endian this is a no-op. On little endian the bytes are swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"big\") {
    assert_eq!(n.to_be(), n)
} else {
    assert_eq!(n.to_be(), n.swap_bytes())
}",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn to_be(self) -> Self { // or not to be?
                #[cfg(target_endian = "big")]
                {
                    self
                }
                #[cfg(not(target_endian = "big"))]
                {
                    self.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn to_be(self) -> Self { // or not to be?
                if cfg!(target_endian = "big") { self } else { self.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Converts `self` to little endian from the target's endianness.

On little endian this is a no-op. On big endian the bytes are swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"little\") {
    assert_eq!(n.to_le(), n)
} else {
    assert_eq!(n.to_le(), n.swap_bytes())
}",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn to_le(self) -> Self {
                #[cfg(target_endian = "little")]
                {
                    self
                }
                #[cfg(not(target_endian = "little"))]
                {
                    self.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn to_le(self) -> Self {
                if cfg!(target_endian = "little") { self } else { self.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Checked integer addition. Computes `self + rhs`, returning `None`
if overflow occurred.

# Examples

Basic usage:

```
", $Feature, "assert_eq!((", stringify!($SelfT),
"::max_value() - 2).checked_add(1), Some(", stringify!($SelfT), "::max_value() - 1));
assert_eq!((", stringify!($SelfT), "::max_value() - 2).checked_add(3), None);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_add(self, rhs: Self) -> Option<Self> {
                let (a, b) = self.overflowing_add(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked integer subtraction. Computes `self - rhs`, returning `None` if
overflow occurred.

# Examples

Basic usage:

```
", $Feature, "assert_eq!((", stringify!($SelfT),
"::min_value() + 2).checked_sub(1), Some(", stringify!($SelfT), "::min_value() + 1));
assert_eq!((", stringify!($SelfT), "::min_value() + 2).checked_sub(3), None);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_sub(self, rhs: Self) -> Option<Self> {
                let (a, b) = self.overflowing_sub(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked integer multiplication. Computes `self * rhs`, returning `None` if
overflow occurred.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT),
"::max_value().checked_mul(1), Some(", stringify!($SelfT), "::max_value()));
assert_eq!(", stringify!($SelfT), "::max_value().checked_mul(2), None);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_mul(self, rhs: Self) -> Option<Self> {
                let (a, b) = self.overflowing_mul(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked integer division. Computes `self / rhs`, returning `None` if `rhs == 0`
or the division results in overflow.

# Examples

Basic usage:

```
", $Feature, "assert_eq!((", stringify!($SelfT),
"::min_value() + 1).checked_div(-1), Some(", stringify!($Max), "));
assert_eq!(", stringify!($SelfT), "::min_value().checked_div(-1), None);
assert_eq!((1", stringify!($SelfT), ").checked_div(0), None);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_div(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::min_value() && rhs == -1) {
                    None
                } else {
                    Some(unsafe { intrinsics::unchecked_div(self, rhs) })
                }
            }
        }

        doc_comment! {
            concat!("Checked Euclidean division. Computes `self.div_euc(rhs)`,
returning `None` if `rhs == 0` or the division results in overflow.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!((", stringify!($SelfT),
"::min_value() + 1).checked_div_euc(-1), Some(", stringify!($Max), "));
assert_eq!(", stringify!($SelfT), "::min_value().checked_div_euc(-1), None);
assert_eq!((1", stringify!($SelfT), ").checked_div_euc(0), None);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn checked_div_euc(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::min_value() && rhs == -1) {
                    None
                } else {
                    Some(self.div_euc(rhs))
                }
            }
        }

        doc_comment! {
            concat!("Checked integer remainder. Computes `self % rhs`, returning `None` if
`rhs == 0` or the division results in overflow.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".checked_rem(2), Some(1));
assert_eq!(5", stringify!($SelfT), ".checked_rem(0), None);
assert_eq!(", stringify!($SelfT), "::MIN.checked_rem(-1), None);",
$EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_rem(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::min_value() && rhs == -1) {
                    None
                } else {
                    Some(unsafe { intrinsics::unchecked_rem(self, rhs) })
                }
            }
        }

        doc_comment! {
            concat!("Checked Euclidean modulo. Computes `self.mod_euc(rhs)`, returning `None` if
`rhs == 0` or the division results in overflow.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".checked_mod_euc(2), Some(1));
assert_eq!(5", stringify!($SelfT), ".checked_mod_euc(0), None);
assert_eq!(", stringify!($SelfT), "::MIN.checked_mod_euc(-1), None);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn checked_mod_euc(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::min_value() && rhs == -1) {
                    None
                } else {
                    Some(self.mod_euc(rhs))
                }
            }
        }

        doc_comment! {
            concat!("Checked negation. Computes `-self`, returning `None` if `self == MIN`.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".checked_neg(), Some(-5));
assert_eq!(", stringify!($SelfT), "::MIN.checked_neg(), None);",
$EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_neg(self) -> Option<Self> {
                let (a, b) = self.overflowing_neg();
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked shift left. Computes `self << rhs`, returning `None` if `rhs` is larger
than or equal to the number of bits in `self`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0x1", stringify!($SelfT), ".checked_shl(4), Some(0x10));
assert_eq!(0x1", stringify!($SelfT), ".checked_shl(129), None);",
$EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_shl(self, rhs: u32) -> Option<Self> {
                let (a, b) = self.overflowing_shl(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked shift right. Computes `self >> rhs`, returning `None` if `rhs` is
larger than or equal to the number of bits in `self`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0x10", stringify!($SelfT), ".checked_shr(4), Some(0x1));
assert_eq!(0x10", stringify!($SelfT), ".checked_shr(128), None);",
$EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_shr(self, rhs: u32) -> Option<Self> {
                let (a, b) = self.overflowing_shr(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked absolute value. Computes `self.abs()`, returning `None` if
`self == MIN`.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!((-5", stringify!($SelfT), ").checked_abs(), Some(5));
assert_eq!(", stringify!($SelfT), "::MIN.checked_abs(), None);",
$EndFeature, "
```"),
            #[stable(feature = "no_panic_abs", since = "1.13.0")]
            #[inline]
            pub fn checked_abs(self) -> Option<Self> {
                if self.is_negative() {
                    self.checked_neg()
                } else {
                    Some(self)
                }
            }
        }

        doc_comment! {
            concat!("Checked exponentiation. Computes `self.pow(exp)`, returning `None` if
overflow occurred.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "assert_eq!(8", stringify!($SelfT), ".checked_pow(2), Some(64));
assert_eq!(", stringify!($SelfT), "::max_value().checked_pow(2), None);",
$EndFeature, "
```"),

            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn checked_pow(self, mut exp: u32) -> Option<Self> {
                let mut base = self;
                let mut acc: Self = 1;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        acc = acc.checked_mul(base)?;
                    }
                    exp /= 2;
                    base = base.checked_mul(base)?;
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    acc = acc.checked_mul(base)?;
                }

                Some(acc)
            }
        }

        doc_comment! {
            concat!("Saturating integer addition. Computes `self + rhs`, saturating at the numeric
bounds instead of overflowing.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".saturating_add(1), 101);
assert_eq!(", stringify!($SelfT), "::max_value().saturating_add(100), ", stringify!($SelfT),
"::max_value());",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn saturating_add(self, rhs: Self) -> Self {
                match self.checked_add(rhs) {
                    Some(x) => x,
                    None if rhs >= 0 => Self::max_value(),
                    None => Self::min_value(),
                }
            }
        }

        doc_comment! {
            concat!("Saturating integer subtraction. Computes `self - rhs`, saturating at the
numeric bounds instead of overflowing.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".saturating_sub(127), -27);
assert_eq!(", stringify!($SelfT), "::min_value().saturating_sub(100), ", stringify!($SelfT),
"::min_value());",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn saturating_sub(self, rhs: Self) -> Self {
                match self.checked_sub(rhs) {
                    Some(x) => x,
                    None if rhs >= 0 => Self::min_value(),
                    None => Self::max_value(),
                }
            }
        }

        doc_comment! {
            concat!("Saturating integer multiplication. Computes `self * rhs`, saturating at the
numeric bounds instead of overflowing.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(10", stringify!($SelfT), ".saturating_mul(12), 120);
assert_eq!(", stringify!($SelfT), "::MAX.saturating_mul(10), ", stringify!($SelfT), "::MAX);
assert_eq!(", stringify!($SelfT), "::MIN.saturating_mul(10), ", stringify!($SelfT), "::MIN);",
$EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn saturating_mul(self, rhs: Self) -> Self {
                self.checked_mul(rhs).unwrap_or_else(|| {
                    if (self < 0 && rhs < 0) || (self > 0 && rhs > 0) {
                        Self::max_value()
                    } else {
                        Self::min_value()
                    }
                })
            }
        }

        doc_comment! {
            concat!("Saturating integer exponentiation. Computes `self.pow(exp)`,
saturating at the numeric bounds instead of overflowing.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!((-4", stringify!($SelfT), ").saturating_pow(3), -64);
assert_eq!(", stringify!($SelfT), "::MIN.saturating_pow(2), ", stringify!($SelfT), "::MAX);
assert_eq!(", stringify!($SelfT), "::MIN.saturating_pow(3), ", stringify!($SelfT), "::MIN);",
$EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn saturating_pow(self, exp: u32) -> Self {
                match self.checked_pow(exp) {
                    Some(x) => x,
                    None if self < 0 && exp % 2 == 1 => Self::min_value(),
                    None => Self::max_value(),
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) addition. Computes `self + rhs`, wrapping around at the
boundary of the type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_add(27), 127);
assert_eq!(", stringify!($SelfT), "::max_value().wrapping_add(2), ", stringify!($SelfT),
"::min_value() + 1);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn wrapping_add(self, rhs: Self) -> Self {
                unsafe {
                    intrinsics::overflowing_add(self, rhs)
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) subtraction. Computes `self - rhs`, wrapping around at the
boundary of the type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0", stringify!($SelfT), ".wrapping_sub(127), -127);
assert_eq!((-2", stringify!($SelfT), ").wrapping_sub(", stringify!($SelfT), "::max_value()), ",
stringify!($SelfT), "::max_value());",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn wrapping_sub(self, rhs: Self) -> Self {
                unsafe {
                    intrinsics::overflowing_sub(self, rhs)
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) multiplication. Computes `self * rhs`, wrapping around at
the boundary of the type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(10", stringify!($SelfT), ".wrapping_mul(12), 120);
assert_eq!(11i8.wrapping_mul(12), -124);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn wrapping_mul(self, rhs: Self) -> Self {
                unsafe {
                    intrinsics::overflowing_mul(self, rhs)
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) division. Computes `self / rhs`, wrapping around at the
boundary of the type.

The only case where such wrapping can occur is when one divides `MIN / -1` on a signed type (where
`MIN` is the negative minimal value for the type); this is equivalent to `-MIN`, a positive value
that is too large to represent in the type. In such a case, this function returns `MIN` itself.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_div(10), 10);
assert_eq!((-128i8).wrapping_div(-1), -128);",
$EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_div(self, rhs: Self) -> Self {
                self.overflowing_div(rhs).0
            }
        }

        doc_comment! {
            concat!("Wrapping Euclidean division. Computes `self.div_euc(rhs)`,
wrapping around at the boundary of the type.

Wrapping will only occur in `MIN / -1` on a signed type (where `MIN` is the negative minimal value
for the type). This is equivalent to `-MIN`, a positive value that is too large to represent in the
type. In this case, this method returns `MIN` itself.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(100", stringify!($SelfT), ".wrapping_div_euc(10), 10);
assert_eq!((-128i8).wrapping_div_euc(-1), -128);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn wrapping_div_euc(self, rhs: Self) -> Self {
                self.overflowing_div_euc(rhs).0
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) remainder. Computes `self % rhs`, wrapping around at the
boundary of the type.

Such wrap-around never actually occurs mathematically; implementation artifacts make `x % y`
invalid for `MIN / -1` on a signed type (where `MIN` is the negative minimal value). In such a case,
this function returns `0`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_rem(10), 0);
assert_eq!((-128i8).wrapping_rem(-1), 0);",
$EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_rem(self, rhs: Self) -> Self {
                self.overflowing_rem(rhs).0
            }
        }

        doc_comment! {
            concat!("Wrapping Euclidean modulo. Computes `self.mod_euc(rhs)`, wrapping around at the
boundary of the type.

Wrapping will only occur in `MIN % -1` on a signed type (where `MIN` is the negative minimal value
for the type). In this case, this method returns 0.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(100", stringify!($SelfT), ".wrapping_mod_euc(10), 0);
assert_eq!((-128i8).wrapping_mod_euc(-1), 0);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn wrapping_mod_euc(self, rhs: Self) -> Self {
                self.overflowing_mod_euc(rhs).0
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) negation. Computes `-self`, wrapping around at the boundary
of the type.

The only case where such wrapping can occur is when one negates `MIN` on a signed type (where `MIN`
is the negative minimal value for the type); this is a positive value that is too large to represent
in the type. In such a case, this function returns `MIN` itself.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_neg(), -100);
assert_eq!(", stringify!($SelfT), "::min_value().wrapping_neg(), ", stringify!($SelfT),
"::min_value());",
$EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_neg(self) -> Self {
                self.overflowing_neg().0
            }
        }

        doc_comment! {
            concat!("Panic-free bitwise shift-left; yields `self << mask(rhs)`, where `mask` removes
any high-order bits of `rhs` that would cause the shift to exceed the bitwidth of the type.

Note that this is *not* the same as a rotate-left; the RHS of a wrapping shift-left is restricted to
the range of the type, rather than the bits shifted out of the LHS being returned to the other end.
The primitive integer types all implement a `rotate_left` function, which may be what you want
instead.

# Examples

Basic usage:

```
", $Feature, "assert_eq!((-1", stringify!($SelfT), ").wrapping_shl(7), -128);
assert_eq!((-1", stringify!($SelfT), ").wrapping_shl(128), -1);",
$EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_shl(self, rhs: u32) -> Self {
                unsafe {
                    intrinsics::unchecked_shl(self, (rhs & ($BITS - 1)) as $SelfT)
                }
            }
        }

        doc_comment! {
            concat!("Panic-free bitwise shift-right; yields `self >> mask(rhs)`, where `mask`
removes any high-order bits of `rhs` that would cause the shift to exceed the bitwidth of the type.

Note that this is *not* the same as a rotate-right; the RHS of a wrapping shift-right is restricted
to the range of the type, rather than the bits shifted out of the LHS being returned to the other
end. The primitive integer types all implement a `rotate_right` function, which may be what you want
instead.

# Examples

Basic usage:

```
", $Feature, "assert_eq!((-128", stringify!($SelfT), ").wrapping_shr(7), -1);
assert_eq!((-128i16).wrapping_shr(64), -128);",
$EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_shr(self, rhs: u32) -> Self {
                unsafe {
                    intrinsics::unchecked_shr(self, (rhs & ($BITS - 1)) as $SelfT)
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) absolute value. Computes `self.abs()`, wrapping around at
the boundary of the type.

The only case where such wrapping can occur is when one takes the absolute value of the negative
minimal value for the type this is a positive value that is too large to represent in the type. In
such a case, this function returns `MIN` itself.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_abs(), 100);
assert_eq!((-100", stringify!($SelfT), ").wrapping_abs(), 100);
assert_eq!(", stringify!($SelfT), "::min_value().wrapping_abs(), ", stringify!($SelfT),
"::min_value());
assert_eq!((-128i8).wrapping_abs() as u8, 128);",
$EndFeature, "
```"),
            #[stable(feature = "no_panic_abs", since = "1.13.0")]
            #[inline]
            pub fn wrapping_abs(self) -> Self {
                if self.is_negative() {
                    self.wrapping_neg()
                } else {
                    self
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) exponentiation. Computes `self.pow(exp)`,
wrapping around at the boundary of the type.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "assert_eq!(3", stringify!($SelfT), ".wrapping_pow(4), 81);
assert_eq!(3i8.wrapping_pow(5), -13);
assert_eq!(3i8.wrapping_pow(6), -39);",
$EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn wrapping_pow(self, mut exp: u32) -> Self {
                let mut base = self;
                let mut acc: Self = 1;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        acc = acc.wrapping_mul(base);
                    }
                    exp /= 2;
                    base = base.wrapping_mul(base);
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    acc = acc.wrapping_mul(base);
                }

                acc
            }
        }

        doc_comment! {
            concat!("Calculates `self` + `rhs`

Returns a tuple of the addition along with a boolean indicating whether an arithmetic overflow would
occur. If an overflow would have occurred then the wrapped value is returned.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_add(2), (7, false));
assert_eq!(", stringify!($SelfT), "::MAX.overflowing_add(1), (", stringify!($SelfT),
"::MIN, true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_add(self, rhs: Self) -> (Self, bool) {
                let (a, b) = unsafe {
                    intrinsics::add_with_overflow(self as $ActualT,
                                                  rhs as $ActualT)
                };
                (a as Self, b)
            }
        }

        doc_comment! {
            concat!("Calculates `self` - `rhs`

Returns a tuple of the subtraction along with a boolean indicating whether an arithmetic overflow
would occur. If an overflow would have occurred then the wrapped value is returned.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_sub(2), (3, false));
assert_eq!(", stringify!($SelfT), "::MIN.overflowing_sub(1), (", stringify!($SelfT),
"::MAX, true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
                let (a, b) = unsafe {
                    intrinsics::sub_with_overflow(self as $ActualT,
                                                  rhs as $ActualT)
                };
                (a as Self, b)
            }
        }

        doc_comment! {
            concat!("Calculates the multiplication of `self` and `rhs`.

Returns a tuple of the multiplication along with a boolean indicating whether an arithmetic overflow
would occur. If an overflow would have occurred then the wrapped value is returned.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(5", stringify!($SelfT), ".overflowing_mul(2), (10, false));
assert_eq!(1_000_000_000i32.overflowing_mul(10), (1410065408, true));",
$EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
                let (a, b) = unsafe {
                    intrinsics::mul_with_overflow(self as $ActualT,
                                                  rhs as $ActualT)
                };
                (a as Self, b)
            }
        }

        doc_comment! {
            concat!("Calculates the divisor when `self` is divided by `rhs`.

Returns a tuple of the divisor along with a boolean indicating whether an arithmetic overflow would
occur. If an overflow would occur then self is returned.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_div(2), (2, false));
assert_eq!(", stringify!($SelfT), "::MIN.overflowing_div(-1), (", stringify!($SelfT),
"::MIN, true));",
$EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_div(self, rhs: Self) -> (Self, bool) {
                if self == Self::min_value() && rhs == -1 {
                    (self, true)
                } else {
                    (self / rhs, false)
                }
            }
        }

        doc_comment! {
            concat!("Calculates the quotient of Euclidean division `self.div_euc(rhs)`.

Returns a tuple of the divisor along with a boolean indicating whether an arithmetic overflow would
occur. If an overflow would occur then `self` is returned.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_div_euc(2), (2, false));
assert_eq!(", stringify!($SelfT), "::MIN.overflowing_div_euc(-1), (", stringify!($SelfT),
"::MIN, true));
```"),
            #[inline]
            #[unstable(feature = "euclidean_division", issue = "49048")]
            pub fn overflowing_div_euc(self, rhs: Self) -> (Self, bool) {
                if self == Self::min_value() && rhs == -1 {
                    (self, true)
                } else {
                    (self.div_euc(rhs), false)
                }
            }
        }

        doc_comment! {
            concat!("Calculates the remainder when `self` is divided by `rhs`.

Returns a tuple of the remainder after dividing along with a boolean indicating whether an
arithmetic overflow would occur. If an overflow would occur then 0 is returned.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_rem(2), (1, false));
assert_eq!(", stringify!($SelfT), "::MIN.overflowing_rem(-1), (0, true));",
$EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_rem(self, rhs: Self) -> (Self, bool) {
                if self == Self::min_value() && rhs == -1 {
                    (0, true)
                } else {
                    (self % rhs, false)
                }
            }
        }


        doc_comment! {
            concat!("Calculates the remainder `self.mod_euc(rhs)` by Euclidean division.

Returns a tuple of the remainder after dividing along with a boolean indicating whether an
arithmetic overflow would occur. If an overflow would occur then 0 is returned.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_mod_euc(2), (1, false));
assert_eq!(", stringify!($SelfT), "::MIN.overflowing_mod_euc(-1), (0, true));
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn overflowing_mod_euc(self, rhs: Self) -> (Self, bool) {
                if self == Self::min_value() && rhs == -1 {
                    (0, true)
                } else {
                    (self.mod_euc(rhs), false)
                }
            }
        }


        doc_comment! {
            concat!("Negates self, overflowing if this is equal to the minimum value.

Returns a tuple of the negated version of self along with a boolean indicating whether an overflow
happened. If `self` is the minimum value (e.g. `i32::MIN` for values of type `i32`), then the
minimum value will be returned again and `true` will be returned for an overflow happening.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(2", stringify!($SelfT), ".overflowing_neg(), (-2, false));
assert_eq!(", stringify!($SelfT), "::MIN.overflowing_neg(), (", stringify!($SelfT),
"::MIN, true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_neg(self) -> (Self, bool) {
                if self == Self::min_value() {
                    (Self::min_value(), true)
                } else {
                    (-self, false)
                }
            }
        }

        doc_comment! {
            concat!("Shifts self left by `rhs` bits.

Returns a tuple of the shifted version of self along with a boolean indicating whether the shift
value was larger than or equal to the number of bits. If the shift value is too large, then value is
masked (N-1) where N is the number of bits, and this value is then used to perform the shift.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0x1", stringify!($SelfT),".overflowing_shl(4), (0x10, false));
assert_eq!(0x1i32.overflowing_shl(36), (0x10, true));",
$EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_shl(self, rhs: u32) -> (Self, bool) {
                (self.wrapping_shl(rhs), (rhs > ($BITS - 1)))
            }
        }

        doc_comment! {
            concat!("Shifts self right by `rhs` bits.

Returns a tuple of the shifted version of self along with a boolean indicating whether the shift
value was larger than or equal to the number of bits. If the shift value is too large, then value is
masked (N-1) where N is the number of bits, and this value is then used to perform the shift.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0x10", stringify!($SelfT), ".overflowing_shr(4), (0x1, false));
assert_eq!(0x10i32.overflowing_shr(36), (0x1, true));",
$EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_shr(self, rhs: u32) -> (Self, bool) {
                (self.wrapping_shr(rhs), (rhs > ($BITS - 1)))
            }
        }

        doc_comment! {
            concat!("Computes the absolute value of `self`.

Returns a tuple of the absolute version of self along with a boolean indicating whether an overflow
happened. If self is the minimum value (e.g. ", stringify!($SelfT), "::MIN for values of type
 ", stringify!($SelfT), "), then the minimum value will be returned again and true will be returned
for an overflow happening.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(10", stringify!($SelfT), ".overflowing_abs(), (10, false));
assert_eq!((-10", stringify!($SelfT), ").overflowing_abs(), (10, false));
assert_eq!((", stringify!($SelfT), "::min_value()).overflowing_abs(), (", stringify!($SelfT),
"::min_value(), true));",
$EndFeature, "
```"),
            #[stable(feature = "no_panic_abs", since = "1.13.0")]
            #[inline]
            pub fn overflowing_abs(self) -> (Self, bool) {
                if self.is_negative() {
                    self.overflowing_neg()
                } else {
                    (self, false)
                }
            }
        }

        doc_comment! {
            concat!("Raises self to the power of `exp`, using exponentiation by squaring.

Returns a tuple of the exponentiation along with a bool indicating
whether an overflow happened.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "assert_eq!(3", stringify!($SelfT), ".overflowing_pow(4), (81, false));
assert_eq!(3i8.overflowing_pow(5), (-13, true));",
$EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn overflowing_pow(self, mut exp: u32) -> (Self, bool) {
                let mut base = self;
                let mut acc: Self = 1;
                let mut overflown = false;
                // Scratch space for storing results of overflowing_mul.
                let mut r;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        r = acc.overflowing_mul(base);
                        acc = r.0;
                        overflown |= r.1;
                    }
                    exp /= 2;
                    r = base.overflowing_mul(base);
                    base = r.0;
                    overflown |= r.1;
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    r = acc.overflowing_mul(base);
                    acc = r.0;
                    overflown |= r.1;
                }

                (acc, overflown)
            }
        }

        doc_comment! {
            concat!("Raises self to the power of `exp`, using exponentiation by squaring.

# Examples

Basic usage:

```
", $Feature, "let x: ", stringify!($SelfT), " = 2; // or any other integer type

assert_eq!(x.pow(5), 32);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            #[rustc_inherit_overflow_checks]
            pub fn pow(self, mut exp: u32) -> Self {
                let mut base = self;
                let mut acc = 1;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        acc = acc * base;
                    }
                    exp /= 2;
                    base = base * base;
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    acc = acc * base;
                }

                acc
            }
        }

        doc_comment! {
            concat!("Calculates the quotient of Euclidean division of `self` by `rhs`.

This computes the integer `n` such that `self = n * rhs + self.mod_euc(rhs)`.
In other words, the result is `self / rhs` rounded to the integer `n`
such that `self >= n * rhs`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
let a: ", stringify!($SelfT), " = 7; // or any other integer type
let b = 4;

assert_eq!(a.div_euc(b), 1); // 7 >= 4 * 1
assert_eq!(a.div_euc(-b), -1); // 7 >= -4 * -1
assert_eq!((-a).div_euc(b), -2); // -7 >= 4 * -2
assert_eq!((-a).div_euc(-b), 2); // -7 >= -4 * 2
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            #[rustc_inherit_overflow_checks]
            pub fn div_euc(self, rhs: Self) -> Self {
                let q = self / rhs;
                if self % rhs < 0 {
                    return if rhs > 0 { q - 1 } else { q + 1 }
                }
                q
            }
        }


        doc_comment! {
            concat!("Calculates the remainder `self mod rhs` by Euclidean division.

In particular, the result `n` satisfies `0 <= n < rhs.abs()`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
let a: ", stringify!($SelfT), " = 7; // or any other integer type
let b = 4;

assert_eq!(a.mod_euc(b), 3);
assert_eq!((-a).mod_euc(b), 1);
assert_eq!(a.mod_euc(-b), 3);
assert_eq!((-a).mod_euc(-b), 1);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            #[rustc_inherit_overflow_checks]
            pub fn mod_euc(self, rhs: Self) -> Self {
                let r = self % rhs;
                if r < 0 {
                    if rhs < 0 {
                        r - rhs
                    } else {
                        r + rhs
                    }
                } else {
                    r
                }
            }
        }

        doc_comment! {
            concat!("Computes the absolute value of `self`.

# Overflow behavior

The absolute value of `", stringify!($SelfT), "::min_value()` cannot be represented as an
`", stringify!($SelfT), "`, and attempting to calculate it will cause an overflow. This means that
code in debug mode will trigger a panic on this case and optimized code will return `",
stringify!($SelfT), "::min_value()` without a panic.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(10", stringify!($SelfT), ".abs(), 10);
assert_eq!((-10", stringify!($SelfT), ").abs(), 10);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            #[rustc_inherit_overflow_checks]
            pub fn abs(self) -> Self {
                if self.is_negative() {
                    // Note that the #[inline] above means that the overflow
                    // semantics of this negation depend on the crate we're being
                    // inlined into.
                    -self
                } else {
                    self
                }
            }
        }

        doc_comment! {
            concat!("Returns a number representing sign of `self`.

 - `0` if the number is zero
 - `1` if the number is positive
 - `-1` if the number is negative

# Examples

Basic usage:

```
", $Feature, "assert_eq!(10", stringify!($SelfT), ".signum(), 1);
assert_eq!(0", stringify!($SelfT), ".signum(), 0);
assert_eq!((-10", stringify!($SelfT), ").signum(), -1);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn signum(self) -> Self {
                match self {
                    n if n > 0 =>  1,
                    0          =>  0,
                    _          => -1,
                }
            }
        }

        doc_comment! {
            concat!("Returns `true` if `self` is positive and `false` if the number is zero or
negative.

# Examples

Basic usage:

```
", $Feature, "assert!(10", stringify!($SelfT), ".is_positive());
assert!(!(-10", stringify!($SelfT), ").is_positive());",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn is_positive(self) -> bool { self > 0 }
        }

        doc_comment! {
            concat!("Returns `true` if `self` is negative and `false` if the number is zero or
positive.

# Examples

Basic usage:

```
", $Feature, "assert!((-10", stringify!($SelfT), ").is_negative());
assert!(!10", stringify!($SelfT), ".is_negative());",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn is_negative(self) -> bool { self < 0 }
        }

        /// Return the memory representation of this integer as a byte array.
        ///
        /// The target platform’s native endianness is used.
        /// Portable code likely wants to use this after [`to_be`] or [`to_le`].
        ///
        /// [`to_be`]: #method.to_be
        /// [`to_le`]: #method.to_le
        ///
        /// # Examples
        ///
        /// ```
        /// #![feature(int_to_from_bytes)]
        ///
        /// let bytes = i32::min_value().to_be().to_bytes();
        /// assert_eq!(bytes, [0x80, 0, 0, 0]);
        /// ```
        #[unstable(feature = "int_to_from_bytes", issue = "49792")]
        #[inline]
        pub fn to_bytes(self) -> [u8; mem::size_of::<Self>()] {
            unsafe { mem::transmute(self) }
        }

        /// Create an integer value from its memory representation as a byte array.
        ///
        /// The target platform’s native endianness is used.
        /// Portable code likely wants to use [`from_be`] or [`from_le`] after this.
        ///
        /// [`from_be`]: #method.from_be
        /// [`from_le`]: #method.from_le
        ///
        /// # Examples
        ///
        /// ```
        /// #![feature(int_to_from_bytes)]
        ///
        /// let int = i32::from_be(i32::from_bytes([0x80, 0, 0, 0]));
        /// assert_eq!(int, i32::min_value());
        /// ```
        #[unstable(feature = "int_to_from_bytes", issue = "49792")]
        #[inline]
        pub fn from_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self {
            unsafe { mem::transmute(bytes) }
        }
    }
}

#[lang = "i8"]
impl i8 {
    int_impl! { i8, i8, u8, 8, -128, 127, "", "" }
}

#[lang = "i16"]
impl i16 {
    int_impl! { i16, i16, u16, 16, -32768, 32767, "", "" }
}

#[lang = "i32"]
impl i32 {
    int_impl! { i32, i32, u32, 32, -2147483648, 2147483647, "", "" }
}

#[lang = "i64"]
impl i64 {
    int_impl! { i64, i64, u64, 64, -9223372036854775808, 9223372036854775807, "", "" }
}

#[lang = "i128"]
impl i128 {
    int_impl! { i128, i128, u128, 128, -170141183460469231731687303715884105728,
        170141183460469231731687303715884105727, "", "" }
}

#[cfg(target_pointer_width = "16")]
#[lang = "isize"]
impl isize {
    int_impl! { isize, i16, u16, 16, -32768, 32767, "", "" }
}

#[cfg(target_pointer_width = "32")]
#[lang = "isize"]
impl isize {
    int_impl! { isize, i32, u32, 32, -2147483648, 2147483647, "", "" }
}

#[cfg(target_pointer_width = "64")]
#[lang = "isize"]
impl isize {
    int_impl! { isize, i64, u64, 64, -9223372036854775808, 9223372036854775807, "", "" }
}

// Emits the correct `cttz` call, depending on the size of the type.
macro_rules! uint_cttz_call {
    // As of LLVM 3.6 the codegen for the zero-safe cttz8 intrinsic
    // emits two conditional moves on x86_64. By promoting the value to
    // u16 and setting bit 8, we get better code without any conditional
    // operations.
    // FIXME: There's a LLVM patch (http://reviews.llvm.org/D9284)
    // pending, remove this workaround once LLVM generates better code
    // for cttz8.
    ($value:expr, 8) => { intrinsics::cttz($value as u16 | 0x100) };
    ($value:expr, $_BITS:expr) => { intrinsics::cttz($value) }
}

// `Int` + `UnsignedInt` implemented for unsigned integers
macro_rules! uint_impl {
    ($SelfT:ty, $ActualT:ty, $BITS:expr, $MaxV:expr, $Feature:expr, $EndFeature:expr) => {
        doc_comment! {
            concat!("Returns the smallest value that can be represented by this integer type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::min_value(), 0);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub const fn min_value() -> Self { 0 }
        }

        doc_comment! {
            concat!("Returns the largest value that can be represented by this integer type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::max_value(), ",
stringify!($MaxV), ");", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub const fn max_value() -> Self { !0 }
        }

        doc_comment! {
            concat!("Converts a string slice in a given base to an integer.

The string is expected to be an optional `+` sign
followed by digits.
Leading and trailing whitespace represent an error.
Digits are a subset of these characters, depending on `radix`:

* `0-9`
* `a-z`
* `A-Z`

# Panics

This function panics if `radix` is not in the range from 2 to 36.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::from_str_radix(\"A\", 16), Ok(10));",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
                from_str_radix(src, radix)
            }
        }

        doc_comment! {
            concat!("Returns the number of ones in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "let n = 0b01001100", stringify!($SelfT), ";

assert_eq!(n.count_ones(), 3);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn count_ones(self) -> u32 {
                unsafe { intrinsics::ctpop(self as $ActualT) as u32 }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn count_ones(self) -> u32 {
                unsafe { intrinsics::ctpop(self as $ActualT) as u32 }
            }
        }

        doc_comment! {
            concat!("Returns the number of zeros in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(", stringify!($SelfT), "::max_value().count_zeros(), 0);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn count_zeros(self) -> u32 {
                (!self).count_ones()
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn count_zeros(self) -> u32 {
                (!self).count_ones()
            }
        }

        doc_comment! {
            concat!("Returns the number of leading zeros in the binary representation of `self`.

# Examples

Basic usage:

```
", $Feature, "let n = ", stringify!($SelfT), "::max_value() >> 2;

assert_eq!(n.leading_zeros(), 2);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn leading_zeros(self) -> u32 {
                unsafe { intrinsics::ctlz(self as $ActualT) as u32 }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn leading_zeros(self) -> u32 {
                unsafe { intrinsics::ctlz(self as $ActualT) as u32 }
            }
        }

        doc_comment! {
            concat!("Returns the number of trailing zeros in the binary representation
of `self`.

# Examples

Basic usage:

```
", $Feature, "let n = 0b0101000", stringify!($SelfT), ";

assert_eq!(n.trailing_zeros(), 3);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn trailing_zeros(self) -> u32 {
                unsafe { uint_cttz_call!(self, $BITS) as u32 }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn trailing_zeros(self) -> u32 {
                unsafe { uint_cttz_call!(self, $BITS) as u32 }
            }
        }

        /// Shifts the bits to the left by a specified amount, `n`,
        /// wrapping the truncated bits to the end of the resulting integer.
        ///
        /// Please note this isn't the same operation as `<<`!
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `u64` is used here.
        ///
        /// ```
        /// let n = 0x0123456789ABCDEFu64;
        /// let m = 0x3456789ABCDEF012u64;
        ///
        /// assert_eq!(n.rotate_left(12), m);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[inline]
        pub fn rotate_left(self, n: u32) -> Self {
            // Protect against undefined behaviour for over-long bit shifts
            let n = n % $BITS;
            (self << n) | (self >> (($BITS - n) % $BITS))
        }

        /// Shifts the bits to the right by a specified amount, `n`,
        /// wrapping the truncated bits to the beginning of the resulting
        /// integer.
        ///
        /// Please note this isn't the same operation as `>>`!
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `u64` is used here.
        ///
        /// ```
        /// let n = 0x0123456789ABCDEFu64;
        /// let m = 0xDEF0123456789ABCu64;
        ///
        /// assert_eq!(n.rotate_right(12), m);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[inline]
        pub fn rotate_right(self, n: u32) -> Self {
            // Protect against undefined behaviour for over-long bit shifts
            let n = n % $BITS;
            (self >> n) | (self << (($BITS - n) % $BITS))
        }

        /// Reverses the byte order of the integer.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `u16` is used here.
        ///
        /// ```
        /// let n: u16 = 0b0000000_01010101;
        /// assert_eq!(n, 85);
        ///
        /// let m = n.swap_bytes();
        ///
        /// assert_eq!(m, 0b01010101_00000000);
        /// assert_eq!(m, 21760);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[cfg(not(stage0))]
        #[rustc_const_unstable(feature = "const_int_ops")]
        #[inline]
        pub const fn swap_bytes(self) -> Self {
            unsafe { intrinsics::bswap(self as $ActualT) as Self }
        }

        /// Dummy docs. See !stage0 documentation.
        #[stable(feature = "rust1", since = "1.0.0")]
        #[cfg(stage0)]
        #[inline]
        pub fn swap_bytes(self) -> Self {
            unsafe { intrinsics::bswap(self as $ActualT) as Self }
        }

        /// Reverses the bit pattern of the integer.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `u16` is used here.
        ///
        /// ```
        /// #![feature(reverse_bits)]
        ///
        /// let n: u16 = 0b0000000_01010101;
        /// assert_eq!(n, 85);
        ///
        /// let m = n.reverse_bits();
        ///
        /// assert_eq!(m, 0b10101010_00000000);
        /// assert_eq!(m, 43520);
        /// ```
        #[unstable(feature = "reverse_bits", issue = "48763")]
        #[inline]
        pub fn reverse_bits(self) -> Self {
            unsafe { intrinsics::bitreverse(self as $ActualT) as Self }
        }

        doc_comment! {
            concat!("Converts an integer from big endian to the target's endianness.

On big endian this is a no-op. On little endian the bytes are
swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"big\") {
    assert_eq!(", stringify!($SelfT), "::from_be(n), n)
} else {
    assert_eq!(", stringify!($SelfT), "::from_be(n), n.swap_bytes())
}", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn from_be(x: Self) -> Self {
                #[cfg(target_endian = "big")]
                {
                    x
                }
                #[cfg(not(target_endian = "big"))]
                {
                    x.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn from_be(x: Self) -> Self {
                if cfg!(target_endian = "big") { x } else { x.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Converts an integer from little endian to the target's endianness.

On little endian this is a no-op. On big endian the bytes are
swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"little\") {
    assert_eq!(", stringify!($SelfT), "::from_le(n), n)
} else {
    assert_eq!(", stringify!($SelfT), "::from_le(n), n.swap_bytes())
}", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn from_le(x: Self) -> Self {
                #[cfg(target_endian = "little")]
                {
                    x
                }
                #[cfg(not(target_endian = "little"))]
                {
                    x.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn from_le(x: Self) -> Self {
                if cfg!(target_endian = "little") { x } else { x.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Converts `self` to big endian from the target's endianness.

On big endian this is a no-op. On little endian the bytes are
swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"big\") {
    assert_eq!(n.to_be(), n)
} else {
    assert_eq!(n.to_be(), n.swap_bytes())
}", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn to_be(self) -> Self { // or not to be?
                #[cfg(target_endian = "big")]
                {
                    self
                }
                #[cfg(not(target_endian = "big"))]
                {
                    self.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn to_be(self) -> Self { // or not to be?
                if cfg!(target_endian = "big") { self } else { self.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Converts `self` to little endian from the target's endianness.

On little endian this is a no-op. On big endian the bytes are
swapped.

# Examples

Basic usage:

```
", $Feature, "let n = 0x1A", stringify!($SelfT), ";

if cfg!(target_endian = \"little\") {
    assert_eq!(n.to_le(), n)
} else {
    assert_eq!(n.to_le(), n.swap_bytes())
}", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(not(stage0))]
            #[rustc_const_unstable(feature = "const_int_ops")]
            #[inline]
            pub const fn to_le(self) -> Self {
                #[cfg(target_endian = "little")]
                {
                    self
                }
                #[cfg(not(target_endian = "little"))]
                {
                    self.swap_bytes()
                }
            }
        }

        doc_comment! {
            concat!("Dummy docs. See !stage0 documentation"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[cfg(stage0)]
            #[inline]
            pub fn to_le(self) -> Self {
                if cfg!(target_endian = "little") { self } else { self.swap_bytes() }
            }
        }

        doc_comment! {
            concat!("Checked integer addition. Computes `self + rhs`, returning `None`
if overflow occurred.

# Examples

Basic usage:

```
", $Feature, "assert_eq!((", stringify!($SelfT), "::max_value() - 2).checked_add(1), ",
"Some(", stringify!($SelfT), "::max_value() - 1));
assert_eq!((", stringify!($SelfT), "::max_value() - 2).checked_add(3), None);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_add(self, rhs: Self) -> Option<Self> {
                let (a, b) = self.overflowing_add(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked integer subtraction. Computes `self - rhs`, returning
`None` if overflow occurred.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(1", stringify!($SelfT), ".checked_sub(1), Some(0));
assert_eq!(0", stringify!($SelfT), ".checked_sub(1), None);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_sub(self, rhs: Self) -> Option<Self> {
                let (a, b) = self.overflowing_sub(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked integer multiplication. Computes `self * rhs`, returning
`None` if overflow occurred.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(5", stringify!($SelfT), ".checked_mul(1), Some(5));
assert_eq!(", stringify!($SelfT), "::max_value().checked_mul(2), None);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_mul(self, rhs: Self) -> Option<Self> {
                let (a, b) = self.overflowing_mul(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked integer division. Computes `self / rhs`, returning `None`
if `rhs == 0`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(128", stringify!($SelfT), ".checked_div(2), Some(64));
assert_eq!(1", stringify!($SelfT), ".checked_div(0), None);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn checked_div(self, rhs: Self) -> Option<Self> {
                match rhs {
                    0 => None,
                    rhs => Some(unsafe { intrinsics::unchecked_div(self, rhs) }),
                }
            }
        }

        doc_comment! {
            concat!("Checked Euclidean division. Computes `self.div_euc(rhs)`, returning `None`
if `rhs == 0`.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(128", stringify!($SelfT), ".checked_div(2), Some(64));
assert_eq!(1", stringify!($SelfT), ".checked_div_euc(0), None);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn checked_div_euc(self, rhs: Self) -> Option<Self> {
                if rhs == 0 {
                    None
                } else {
                    Some(self.div_euc(rhs))
                }
            }
        }


        doc_comment! {
            concat!("Checked integer remainder. Computes `self % rhs`, returning `None`
if `rhs == 0`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(5", stringify!($SelfT), ".checked_rem(2), Some(1));
assert_eq!(5", stringify!($SelfT), ".checked_rem(0), None);", $EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_rem(self, rhs: Self) -> Option<Self> {
                if rhs == 0 {
                    None
                } else {
                    Some(unsafe { intrinsics::unchecked_rem(self, rhs) })
                }
            }
        }

        doc_comment! {
            concat!("Checked Euclidean modulo. Computes `self.mod_euc(rhs)`, returning `None`
if `rhs == 0`.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(5", stringify!($SelfT), ".checked_mod_euc(2), Some(1));
assert_eq!(5", stringify!($SelfT), ".checked_mod_euc(0), None);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn checked_mod_euc(self, rhs: Self) -> Option<Self> {
                if rhs == 0 {
                    None
                } else {
                    Some(self.mod_euc(rhs))
                }
            }
        }

        doc_comment! {
            concat!("Checked negation. Computes `-self`, returning `None` unless `self ==
0`.

Note that negating any positive integer will overflow.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0", stringify!($SelfT), ".checked_neg(), Some(0));
assert_eq!(1", stringify!($SelfT), ".checked_neg(), None);", $EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_neg(self) -> Option<Self> {
                let (a, b) = self.overflowing_neg();
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked shift left. Computes `self << rhs`, returning `None`
if `rhs` is larger than or equal to the number of bits in `self`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0x1", stringify!($SelfT), ".checked_shl(4), Some(0x10));
assert_eq!(0x10", stringify!($SelfT), ".checked_shl(129), None);", $EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_shl(self, rhs: u32) -> Option<Self> {
                let (a, b) = self.overflowing_shl(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked shift right. Computes `self >> rhs`, returning `None`
if `rhs` is larger than or equal to the number of bits in `self`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(0x10", stringify!($SelfT), ".checked_shr(4), Some(0x1));
assert_eq!(0x10", stringify!($SelfT), ".checked_shr(129), None);", $EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn checked_shr(self, rhs: u32) -> Option<Self> {
                let (a, b) = self.overflowing_shr(rhs);
                if b {None} else {Some(a)}
            }
        }

        doc_comment! {
            concat!("Checked exponentiation. Computes `self.pow(exp)`, returning `None` if
overflow occurred.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "assert_eq!(2", stringify!($SelfT), ".checked_pow(5), Some(32));
assert_eq!(", stringify!($SelfT), "::max_value().checked_pow(2), None);", $EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn checked_pow(self, mut exp: u32) -> Option<Self> {
                let mut base = self;
                let mut acc: Self = 1;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        acc = acc.checked_mul(base)?;
                    }
                    exp /= 2;
                    base = base.checked_mul(base)?;
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    acc = acc.checked_mul(base)?;
                }

                Some(acc)
            }
        }

        doc_comment! {
            concat!("Saturating integer addition. Computes `self + rhs`, saturating at
the numeric bounds instead of overflowing.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".saturating_add(1), 101);
assert_eq!(200u8.saturating_add(127), 255);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn saturating_add(self, rhs: Self) -> Self {
                match self.checked_add(rhs) {
                    Some(x) => x,
                    None => Self::max_value(),
                }
            }
        }

        doc_comment! {
            concat!("Saturating integer subtraction. Computes `self - rhs`, saturating
at the numeric bounds instead of overflowing.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".saturating_sub(27), 73);
assert_eq!(13", stringify!($SelfT), ".saturating_sub(127), 0);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn saturating_sub(self, rhs: Self) -> Self {
                match self.checked_sub(rhs) {
                    Some(x) => x,
                    None => Self::min_value(),
                }
            }
        }

        doc_comment! {
            concat!("Saturating integer multiplication. Computes `self * rhs`,
saturating at the numeric bounds instead of overflowing.

# Examples

Basic usage:

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(2", stringify!($SelfT), ".saturating_mul(10), 20);
assert_eq!((", stringify!($SelfT), "::MAX).saturating_mul(10), ", stringify!($SelfT),
"::MAX);", $EndFeature, "
```"),
            #[stable(feature = "wrapping", since = "1.7.0")]
            #[inline]
            pub fn saturating_mul(self, rhs: Self) -> Self {
                self.checked_mul(rhs).unwrap_or(Self::max_value())
            }
        }

        doc_comment! {
            concat!("Saturating integer exponentiation. Computes `self.pow(exp)`,
saturating at the numeric bounds instead of overflowing.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(4", stringify!($SelfT), ".saturating_pow(3), 64);
assert_eq!(", stringify!($SelfT), "::MAX.saturating_pow(2), ", stringify!($SelfT), "::MAX);",
$EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn saturating_pow(self, exp: u32) -> Self {
                match self.checked_pow(exp) {
                    Some(x) => x,
                    None => Self::max_value(),
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) addition. Computes `self + rhs`,
wrapping around at the boundary of the type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(200", stringify!($SelfT), ".wrapping_add(55), 255);
assert_eq!(200", stringify!($SelfT), ".wrapping_add(", stringify!($SelfT), "::max_value()), 199);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn wrapping_add(self, rhs: Self) -> Self {
                unsafe {
                    intrinsics::overflowing_add(self, rhs)
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) subtraction. Computes `self - rhs`,
wrapping around at the boundary of the type.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_sub(100), 0);
assert_eq!(100", stringify!($SelfT), ".wrapping_sub(", stringify!($SelfT), "::max_value()), 101);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn wrapping_sub(self, rhs: Self) -> Self {
                unsafe {
                    intrinsics::overflowing_sub(self, rhs)
                }
            }
        }

        /// Wrapping (modular) multiplication. Computes `self *
        /// rhs`, wrapping around at the boundary of the type.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `u8` is used here.
        ///
        /// ```
        /// assert_eq!(10u8.wrapping_mul(12), 120);
        /// assert_eq!(25u8.wrapping_mul(12), 44);
        /// ```
        #[stable(feature = "rust1", since = "1.0.0")]
        #[inline]
        pub fn wrapping_mul(self, rhs: Self) -> Self {
            unsafe {
                intrinsics::overflowing_mul(self, rhs)
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) division. Computes `self / rhs`.
Wrapped division on unsigned types is just normal division.
There's no way wrapping could ever happen.
This function exists, so that all operations
are accounted for in the wrapping operations.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_div(10), 10);", $EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_div(self, rhs: Self) -> Self {
                self / rhs
            }
        }

        doc_comment! {
            concat!("Wrapping Euclidean division. Computes `self.div_euc(rhs)`.
Wrapped division on unsigned types is just normal division.
There's no way wrapping could ever happen.
This function exists, so that all operations
are accounted for in the wrapping operations.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(100", stringify!($SelfT), ".wrapping_div_euc(10), 10);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn wrapping_div_euc(self, rhs: Self) -> Self {
                self / rhs
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) remainder. Computes `self % rhs`.
Wrapped remainder calculation on unsigned types is
just the regular remainder calculation.
There's no way wrapping could ever happen.
This function exists, so that all operations
are accounted for in the wrapping operations.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(100", stringify!($SelfT), ".wrapping_rem(10), 0);", $EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_rem(self, rhs: Self) -> Self {
                self % rhs
            }
        }

        doc_comment! {
            concat!("Wrapping Euclidean modulo. Computes `self.mod_euc(rhs)`.
Wrapped modulo calculation on unsigned types is
just the regular remainder calculation.
There's no way wrapping could ever happen.
This function exists, so that all operations
are accounted for in the wrapping operations.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(100", stringify!($SelfT), ".wrapping_mod_euc(10), 0);
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            pub fn wrapping_mod_euc(self, rhs: Self) -> Self {
                self % rhs
            }
        }

        /// Wrapping (modular) negation. Computes `-self`,
        /// wrapping around at the boundary of the type.
        ///
        /// Since unsigned types do not have negative equivalents
        /// all applications of this function will wrap (except for `-0`).
        /// For values smaller than the corresponding signed type's maximum
        /// the result is the same as casting the corresponding signed value.
        /// Any larger values are equivalent to `MAX + 1 - (val - MAX - 1)` where
        /// `MAX` is the corresponding signed type's maximum.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `i8` is used here.
        ///
        /// ```
        /// assert_eq!(100i8.wrapping_neg(), -100);
        /// assert_eq!((-128i8).wrapping_neg(), -128);
        /// ```
        #[stable(feature = "num_wrapping", since = "1.2.0")]
        #[inline]
        pub fn wrapping_neg(self) -> Self {
            self.overflowing_neg().0
        }

        doc_comment! {
            concat!("Panic-free bitwise shift-left; yields `self << mask(rhs)`,
where `mask` removes any high-order bits of `rhs` that
would cause the shift to exceed the bitwidth of the type.

Note that this is *not* the same as a rotate-left; the
RHS of a wrapping shift-left is restricted to the range
of the type, rather than the bits shifted out of the LHS
being returned to the other end. The primitive integer
types all implement a `rotate_left` function, which may
be what you want instead.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(1", stringify!($SelfT), ".wrapping_shl(7), 128);
assert_eq!(1", stringify!($SelfT), ".wrapping_shl(128), 1);", $EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_shl(self, rhs: u32) -> Self {
                unsafe {
                    intrinsics::unchecked_shl(self, (rhs & ($BITS - 1)) as $SelfT)
                }
            }
        }

        doc_comment! {
            concat!("Panic-free bitwise shift-right; yields `self >> mask(rhs)`,
where `mask` removes any high-order bits of `rhs` that
would cause the shift to exceed the bitwidth of the type.

Note that this is *not* the same as a rotate-right; the
RHS of a wrapping shift-right is restricted to the range
of the type, rather than the bits shifted out of the LHS
being returned to the other end. The primitive integer
types all implement a `rotate_right` function, which may
be what you want instead.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(128", stringify!($SelfT), ".wrapping_shr(7), 1);
assert_eq!(128", stringify!($SelfT), ".wrapping_shr(128), 128);", $EndFeature, "
```"),
            #[stable(feature = "num_wrapping", since = "1.2.0")]
            #[inline]
            pub fn wrapping_shr(self, rhs: u32) -> Self {
                unsafe {
                    intrinsics::unchecked_shr(self, (rhs & ($BITS - 1)) as $SelfT)
                }
            }
        }

        doc_comment! {
            concat!("Wrapping (modular) exponentiation. Computes `self.pow(exp)`,
wrapping around at the boundary of the type.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "assert_eq!(3", stringify!($SelfT), ".wrapping_pow(5), 243);
assert_eq!(3u8.wrapping_pow(6), 217);", $EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn wrapping_pow(self, mut exp: u32) -> Self {
                let mut base = self;
                let mut acc: Self = 1;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        acc = acc.wrapping_mul(base);
                    }
                    exp /= 2;
                    base = base.wrapping_mul(base);
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    acc = acc.wrapping_mul(base);
                }

                acc
            }
        }

        doc_comment! {
            concat!("Calculates `self` + `rhs`

Returns a tuple of the addition along with a boolean indicating
whether an arithmetic overflow would occur. If an overflow would
have occurred then the wrapped value is returned.

# Examples

Basic usage

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_add(2), (7, false));
assert_eq!(", stringify!($SelfT), "::MAX.overflowing_add(1), (0, true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_add(self, rhs: Self) -> (Self, bool) {
                let (a, b) = unsafe {
                    intrinsics::add_with_overflow(self as $ActualT,
                                                  rhs as $ActualT)
                };
                (a as Self, b)
            }
        }

        doc_comment! {
            concat!("Calculates `self` - `rhs`

Returns a tuple of the subtraction along with a boolean indicating
whether an arithmetic overflow would occur. If an overflow would
have occurred then the wrapped value is returned.

# Examples

Basic usage

```
", $Feature, "use std::", stringify!($SelfT), ";

assert_eq!(5", stringify!($SelfT), ".overflowing_sub(2), (3, false));
assert_eq!(0", stringify!($SelfT), ".overflowing_sub(1), (", stringify!($SelfT), "::MAX, true));",
$EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
                let (a, b) = unsafe {
                    intrinsics::sub_with_overflow(self as $ActualT,
                                                  rhs as $ActualT)
                };
                (a as Self, b)
            }
        }

        /// Calculates the multiplication of `self` and `rhs`.
        ///
        /// Returns a tuple of the multiplication along with a boolean
        /// indicating whether an arithmetic overflow would occur. If an
        /// overflow would have occurred then the wrapped value is returned.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// Please note that this example is shared between integer types.
        /// Which explains why `u32` is used here.
        ///
        /// ```
        /// assert_eq!(5u32.overflowing_mul(2), (10, false));
        /// assert_eq!(1_000_000_000u32.overflowing_mul(10), (1410065408, true));
        /// ```
        #[inline]
        #[stable(feature = "wrapping", since = "1.7.0")]
        pub fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
            let (a, b) = unsafe {
                intrinsics::mul_with_overflow(self as $ActualT,
                                              rhs as $ActualT)
            };
            (a as Self, b)
        }

        doc_comment! {
            concat!("Calculates the divisor when `self` is divided by `rhs`.

Returns a tuple of the divisor along with a boolean indicating
whether an arithmetic overflow would occur. Note that for unsigned
integers overflow never occurs, so the second value is always
`false`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage

```
", $Feature, "assert_eq!(5", stringify!($SelfT), ".overflowing_div(2), (2, false));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_div(self, rhs: Self) -> (Self, bool) {
                (self / rhs, false)
            }
        }

        doc_comment! {
            concat!("Calculates the quotient of Euclidean division `self.div_euc(rhs)`.

Returns a tuple of the divisor along with a boolean indicating
whether an arithmetic overflow would occur. Note that for unsigned
integers overflow never occurs, so the second value is always
`false`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage

```
#![feature(euclidean_division)]
assert_eq!(5", stringify!($SelfT), ".overflowing_div_euc(2), (2, false));
```"),
            #[inline]
            #[unstable(feature = "euclidean_division", issue = "49048")]
            pub fn overflowing_div_euc(self, rhs: Self) -> (Self, bool) {
                (self / rhs, false)
            }
        }

        doc_comment! {
            concat!("Calculates the remainder when `self` is divided by `rhs`.

Returns a tuple of the remainder after dividing along with a boolean
indicating whether an arithmetic overflow would occur. Note that for
unsigned integers overflow never occurs, so the second value is
always `false`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage

```
", $Feature, "assert_eq!(5", stringify!($SelfT), ".overflowing_rem(2), (1, false));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_rem(self, rhs: Self) -> (Self, bool) {
                (self % rhs, false)
            }
        }

        doc_comment! {
            concat!("Calculates the remainder `self.mod_euc(rhs)` by Euclidean division.

Returns a tuple of the modulo after dividing along with a boolean
indicating whether an arithmetic overflow would occur. Note that for
unsigned integers overflow never occurs, so the second value is
always `false`.

# Panics

This function will panic if `rhs` is 0.

# Examples

Basic usage

```
#![feature(euclidean_division)]
assert_eq!(5", stringify!($SelfT), ".overflowing_mod_euc(2), (1, false));
```"),
            #[inline]
            #[unstable(feature = "euclidean_division", issue = "49048")]
            pub fn overflowing_mod_euc(self, rhs: Self) -> (Self, bool) {
                (self % rhs, false)
            }
        }

        doc_comment! {
            concat!("Negates self in an overflowing fashion.

Returns `!self + 1` using wrapping operations to return the value
that represents the negation of this unsigned value. Note that for
positive unsigned values overflow always occurs, but negating 0 does
not overflow.

# Examples

Basic usage

```
", $Feature, "assert_eq!(0", stringify!($SelfT), ".overflowing_neg(), (0, false));
assert_eq!(2", stringify!($SelfT), ".overflowing_neg(), (-2i32 as ", stringify!($SelfT),
", true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_neg(self) -> (Self, bool) {
                ((!self).wrapping_add(1), self != 0)
            }
        }

        doc_comment! {
            concat!("Shifts self left by `rhs` bits.

Returns a tuple of the shifted version of self along with a boolean
indicating whether the shift value was larger than or equal to the
number of bits. If the shift value is too large, then value is
masked (N-1) where N is the number of bits, and this value is then
used to perform the shift.

# Examples

Basic usage

```
", $Feature, "assert_eq!(0x1", stringify!($SelfT), ".overflowing_shl(4), (0x10, false));
assert_eq!(0x1", stringify!($SelfT), ".overflowing_shl(132), (0x10, true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_shl(self, rhs: u32) -> (Self, bool) {
                (self.wrapping_shl(rhs), (rhs > ($BITS - 1)))
            }
        }

        doc_comment! {
            concat!("Shifts self right by `rhs` bits.

Returns a tuple of the shifted version of self along with a boolean
indicating whether the shift value was larger than or equal to the
number of bits. If the shift value is too large, then value is
masked (N-1) where N is the number of bits, and this value is then
used to perform the shift.

# Examples

Basic usage

```
", $Feature, "assert_eq!(0x10", stringify!($SelfT), ".overflowing_shr(4), (0x1, false));
assert_eq!(0x10", stringify!($SelfT), ".overflowing_shr(132), (0x1, true));", $EndFeature, "
```"),
            #[inline]
            #[stable(feature = "wrapping", since = "1.7.0")]
            pub fn overflowing_shr(self, rhs: u32) -> (Self, bool) {
                (self.wrapping_shr(rhs), (rhs > ($BITS - 1)))
            }
        }

        doc_comment! {
            concat!("Raises self to the power of `exp`, using exponentiation by squaring.

Returns a tuple of the exponentiation along with a bool indicating
whether an overflow happened.

# Examples

Basic usage:

```
#![feature(no_panic_pow)]
", $Feature, "assert_eq!(3", stringify!($SelfT), ".overflowing_pow(5), (243, false));
assert_eq!(3u8.overflowing_pow(6), (217, true));", $EndFeature, "
```"),
            #[unstable(feature = "no_panic_pow", issue = "48320")]
            #[inline]
            pub fn overflowing_pow(self, mut exp: u32) -> (Self, bool) {
                let mut base = self;
                let mut acc: Self = 1;
                let mut overflown = false;
                // Scratch space for storing results of overflowing_mul.
                let mut r;

                while exp > 1 {
                    if (exp & 1) == 1 {
                        r = acc.overflowing_mul(base);
                        acc = r.0;
                        overflown |= r.1;
                    }
                    exp /= 2;
                    r = base.overflowing_mul(base);
                    base = r.0;
                    overflown |= r.1;
                }

                // Deal with the final bit of the exponent separately, since
                // squaring the base afterwards is not necessary and may cause a
                // needless overflow.
                if exp == 1 {
                    r = acc.overflowing_mul(base);
                    acc = r.0;
                    overflown |= r.1;
                }

                (acc, overflown)
            }
        }

        doc_comment! {
            concat!("Raises self to the power of `exp`, using exponentiation by squaring.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(2", stringify!($SelfT), ".pow(5), 32);", $EndFeature, "
```"),
        #[stable(feature = "rust1", since = "1.0.0")]
        #[inline]
        #[rustc_inherit_overflow_checks]
        pub fn pow(self, mut exp: u32) -> Self {
            let mut base = self;
            let mut acc = 1;

            while exp > 1 {
                if (exp & 1) == 1 {
                    acc = acc * base;
                }
                exp /= 2;
                base = base * base;
            }

            // Deal with the final bit of the exponent separately, since
            // squaring the base afterwards is not necessary and may cause a
            // needless overflow.
            if exp == 1 {
                acc = acc * base;
            }

            acc
        }
    }

            doc_comment! {
            concat!("Performs Euclidean division.

For unsigned types, this is just the same as `self / rhs`.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(7", stringify!($SelfT), ".div_euc(4), 1); // or any other integer type
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            #[rustc_inherit_overflow_checks]
            pub fn div_euc(self, rhs: Self) -> Self {
                self / rhs
            }
        }


        doc_comment! {
            concat!("Calculates the remainder `self mod rhs` by Euclidean division.

For unsigned types, this is just the same as `self % rhs`.

# Examples

Basic usage:

```
#![feature(euclidean_division)]
assert_eq!(7", stringify!($SelfT), ".mod_euc(4), 3); // or any other integer type
```"),
            #[unstable(feature = "euclidean_division", issue = "49048")]
            #[inline]
            #[rustc_inherit_overflow_checks]
            pub fn mod_euc(self, rhs: Self) -> Self {
                self % rhs
            }
        }

        doc_comment! {
            concat!("Returns `true` if and only if `self == 2^k` for some `k`.

# Examples

Basic usage:

```
", $Feature, "assert!(16", stringify!($SelfT), ".is_power_of_two());
assert!(!10", stringify!($SelfT), ".is_power_of_two());", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn is_power_of_two(self) -> bool {
                (self.wrapping_sub(1)) & self == 0 && !(self == 0)
            }
        }

        // Returns one less than next power of two.
        // (For 8u8 next power of two is 8u8 and for 6u8 it is 8u8)
        //
        // 8u8.one_less_than_next_power_of_two() == 7
        // 6u8.one_less_than_next_power_of_two() == 7
        //
        // This method cannot overflow, as in the `next_power_of_two`
        // overflow cases it instead ends up returning the maximum value
        // of the type, and can return 0 for 0.
        #[inline]
        fn one_less_than_next_power_of_two(self) -> Self {
            if self <= 1 { return 0; }

            // Because `p > 0`, it cannot consist entirely of leading zeros.
            // That means the shift is always in-bounds, and some processors
            // (such as intel pre-haswell) have more efficient ctlz
            // intrinsics when the argument is non-zero.
            let p = self - 1;
            let z = unsafe { intrinsics::ctlz_nonzero(p) };
            <$SelfT>::max_value() >> z
        }

        doc_comment! {
            concat!("Returns the smallest power of two greater than or equal to `self`.

When return value overflows (i.e. `self > (1 << (N-1))` for type
`uN`), it panics in debug mode and return value is wrapped to 0 in
release mode (the only situation in which method can return 0).

# Examples

Basic usage:

```
", $Feature, "assert_eq!(2", stringify!($SelfT), ".next_power_of_two(), 2);
assert_eq!(3", stringify!($SelfT), ".next_power_of_two(), 4);", $EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            #[inline]
            pub fn next_power_of_two(self) -> Self {
                // Call the trait to get overflow checks
                ops::Add::add(self.one_less_than_next_power_of_two(), 1)
            }
        }

        doc_comment! {
            concat!("Returns the smallest power of two greater than or equal to `n`. If
the next power of two is greater than the type's maximum value,
`None` is returned, otherwise the power of two is wrapped in `Some`.

# Examples

Basic usage:

```
", $Feature, "assert_eq!(2", stringify!($SelfT),
".checked_next_power_of_two(), Some(2));
assert_eq!(3", stringify!($SelfT), ".checked_next_power_of_two(), Some(4));
assert_eq!(", stringify!($SelfT), "::max_value().checked_next_power_of_two(), None);",
$EndFeature, "
```"),
            #[stable(feature = "rust1", since = "1.0.0")]
            pub fn checked_next_power_of_two(self) -> Option<Self> {
                self.one_less_than_next_power_of_two().checked_add(1)
            }
        }

        doc_comment! {
            concat!("Returns the smallest power of two greater than or equal to `n`. If
the next power of two is greater than the type's maximum value,
the return value is wrapped to `0`.

# Examples

Basic usage:

```
#![feature(wrapping_next_power_of_two)]
", $Feature, "
assert_eq!(2", stringify!($SelfT), ".wrapping_next_power_of_two(), 2);
assert_eq!(3", stringify!($SelfT), ".wrapping_next_power_of_two(), 4);
assert_eq!(", stringify!($SelfT), "::max_value().wrapping_next_power_of_two(), 0);",
$EndFeature, "
```"),
            #[unstable(feature = "wrapping_next_power_of_two", issue = "32463",
                       reason = "needs decision on wrapping behaviour")]
            pub fn wrapping_next_power_of_two(self) -> Self {
                self.one_less_than_next_power_of_two().wrapping_add(1)
            }
        }

        /// Return the memory representation of this integer as a byte array.
        ///
        /// The target platform’s native endianness is used.
        /// Portable code likely wants to use this after [`to_be`] or [`to_le`].
        ///
        /// [`to_be`]: #method.to_be
        /// [`to_le`]: #method.to_le
        ///
        /// # Examples
        ///
        /// ```
        /// #![feature(int_to_from_bytes)]
        ///
        /// let bytes = 0x1234_5678_u32.to_be().to_bytes();
        /// assert_eq!(bytes, [0x12, 0x34, 0x56, 0x78]);
        /// ```
        #[unstable(feature = "int_to_from_bytes", issue = "49792")]
        #[inline]
        pub fn to_bytes(self) -> [u8; mem::size_of::<Self>()] {
            unsafe { mem::transmute(self) }
        }

        /// Create an integer value from its memory representation as a byte array.
        ///
        /// The target platform’s native endianness is used.
        /// Portable code likely wants to use [`to_be`] or [`to_le`] after this.
        ///
        /// [`to_be`]: #method.to_be
        /// [`to_le`]: #method.to_le
        ///
        /// # Examples
        ///
        /// ```
        /// #![feature(int_to_from_bytes)]
        ///
        /// let int = u32::from_be(u32::from_bytes([0x12, 0x34, 0x56, 0x78]));
        /// assert_eq!(int, 0x1234_5678_u32);
        /// ```
        #[unstable(feature = "int_to_from_bytes", issue = "49792")]
        #[inline]
        pub fn from_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self {
            unsafe { mem::transmute(bytes) }
        }
    }
}

#[lang = "u8"]
impl u8 {
    uint_impl! { u8, u8, 8, 255, "", "" }


    /// Checks if the value is within the ASCII range.
    ///
    /// # Examples
    ///
    /// ```
    /// let ascii = 97u8;
    /// let non_ascii = 150u8;
    ///
    /// assert!(ascii.is_ascii());
    /// assert!(!non_ascii.is_ascii());
    /// ```
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn is_ascii(&self) -> bool {
        *self & 128 == 0
    }

    /// Makes a copy of the value in its ASCII upper case equivalent.
    ///
    /// ASCII letters 'a' to 'z' are mapped to 'A' to 'Z',
    /// but non-ASCII letters are unchanged.
    ///
    /// To uppercase the value in-place, use [`make_ascii_uppercase`].
    ///
    /// # Examples
    ///
    /// ```
    /// let lowercase_a = 97u8;
    ///
    /// assert_eq!(65, lowercase_a.to_ascii_uppercase());
    /// ```
    ///
    /// [`make_ascii_uppercase`]: #method.make_ascii_uppercase
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn to_ascii_uppercase(&self) -> u8 {
        ASCII_UPPERCASE_MAP[*self as usize]
    }

    /// Makes a copy of the value in its ASCII lower case equivalent.
    ///
    /// ASCII letters 'A' to 'Z' are mapped to 'a' to 'z',
    /// but non-ASCII letters are unchanged.
    ///
    /// To lowercase the value in-place, use [`make_ascii_lowercase`].
    ///
    /// # Examples
    ///
    /// ```
    /// let uppercase_a = 65u8;
    ///
    /// assert_eq!(97, uppercase_a.to_ascii_lowercase());
    /// ```
    ///
    /// [`make_ascii_lowercase`]: #method.make_ascii_lowercase
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn to_ascii_lowercase(&self) -> u8 {
        ASCII_LOWERCASE_MAP[*self as usize]
    }

    /// Checks that two values are an ASCII case-insensitive match.
    ///
    /// This is equivalent to `to_ascii_lowercase(a) == to_ascii_lowercase(b)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let lowercase_a = 97u8;
    /// let uppercase_a = 65u8;
    ///
    /// assert!(lowercase_a.eq_ignore_ascii_case(&uppercase_a));
    /// ```
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn eq_ignore_ascii_case(&self, other: &u8) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }

    /// Converts this value to its ASCII upper case equivalent in-place.
    ///
    /// ASCII letters 'a' to 'z' are mapped to 'A' to 'Z',
    /// but non-ASCII letters are unchanged.
    ///
    /// To return a new uppercased value without modifying the existing one, use
    /// [`to_ascii_uppercase`].
    ///
    /// # Examples
    ///
    /// ```
    /// let mut byte = b'a';
    ///
    /// byte.make_ascii_uppercase();
    ///
    /// assert_eq!(b'A', byte);
    /// ```
    ///
    /// [`to_ascii_uppercase`]: #method.to_ascii_uppercase
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase();
    }

    /// Converts this value to its ASCII lower case equivalent in-place.
    ///
    /// ASCII letters 'A' to 'Z' are mapped to 'a' to 'z',
    /// but non-ASCII letters are unchanged.
    ///
    /// To return a new lowercased value without modifying the existing one, use
    /// [`to_ascii_lowercase`].
    ///
    /// # Examples
    ///
    /// ```
    /// let mut byte = b'A';
    ///
    /// byte.make_ascii_lowercase();
    ///
    /// assert_eq!(b'a', byte);
    /// ```
    ///
    /// [`to_ascii_lowercase`]: #method.to_ascii_lowercase
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase();
    }

    /// Checks if the value is an ASCII alphabetic character:
    ///
    /// - U+0041 'A' ... U+005A 'Z', or
    /// - U+0061 'a' ... U+007A 'z'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(uppercase_a.is_ascii_alphabetic());
    /// assert!(uppercase_g.is_ascii_alphabetic());
    /// assert!(a.is_ascii_alphabetic());
    /// assert!(g.is_ascii_alphabetic());
    /// assert!(!zero.is_ascii_alphabetic());
    /// assert!(!percent.is_ascii_alphabetic());
    /// assert!(!space.is_ascii_alphabetic());
    /// assert!(!lf.is_ascii_alphabetic());
    /// assert!(!esc.is_ascii_alphabetic());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_alphabetic(&self) -> bool {
        if *self >= 0x80 { return false; }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            L | Lx | U | Ux => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII uppercase character:
    /// U+0041 'A' ... U+005A 'Z'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(uppercase_a.is_ascii_uppercase());
    /// assert!(uppercase_g.is_ascii_uppercase());
    /// assert!(!a.is_ascii_uppercase());
    /// assert!(!g.is_ascii_uppercase());
    /// assert!(!zero.is_ascii_uppercase());
    /// assert!(!percent.is_ascii_uppercase());
    /// assert!(!space.is_ascii_uppercase());
    /// assert!(!lf.is_ascii_uppercase());
    /// assert!(!esc.is_ascii_uppercase());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_uppercase(&self) -> bool {
        if *self >= 0x80 { return false }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            U | Ux => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII lowercase character:
    /// U+0061 'a' ... U+007A 'z'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(!uppercase_a.is_ascii_lowercase());
    /// assert!(!uppercase_g.is_ascii_lowercase());
    /// assert!(a.is_ascii_lowercase());
    /// assert!(g.is_ascii_lowercase());
    /// assert!(!zero.is_ascii_lowercase());
    /// assert!(!percent.is_ascii_lowercase());
    /// assert!(!space.is_ascii_lowercase());
    /// assert!(!lf.is_ascii_lowercase());
    /// assert!(!esc.is_ascii_lowercase());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_lowercase(&self) -> bool {
        if *self >= 0x80 { return false }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            L | Lx => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII alphanumeric character:
    ///
    /// - U+0041 'A' ... U+005A 'Z', or
    /// - U+0061 'a' ... U+007A 'z', or
    /// - U+0030 '0' ... U+0039 '9'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(uppercase_a.is_ascii_alphanumeric());
    /// assert!(uppercase_g.is_ascii_alphanumeric());
    /// assert!(a.is_ascii_alphanumeric());
    /// assert!(g.is_ascii_alphanumeric());
    /// assert!(zero.is_ascii_alphanumeric());
    /// assert!(!percent.is_ascii_alphanumeric());
    /// assert!(!space.is_ascii_alphanumeric());
    /// assert!(!lf.is_ascii_alphanumeric());
    /// assert!(!esc.is_ascii_alphanumeric());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_alphanumeric(&self) -> bool {
        if *self >= 0x80 { return false }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            D | L | Lx | U | Ux => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII decimal digit:
    /// U+0030 '0' ... U+0039 '9'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(!uppercase_a.is_ascii_digit());
    /// assert!(!uppercase_g.is_ascii_digit());
    /// assert!(!a.is_ascii_digit());
    /// assert!(!g.is_ascii_digit());
    /// assert!(zero.is_ascii_digit());
    /// assert!(!percent.is_ascii_digit());
    /// assert!(!space.is_ascii_digit());
    /// assert!(!lf.is_ascii_digit());
    /// assert!(!esc.is_ascii_digit());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_digit(&self) -> bool {
        if *self >= 0x80 { return false }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            D => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII hexadecimal digit:
    ///
    /// - U+0030 '0' ... U+0039 '9', or
    /// - U+0041 'A' ... U+0046 'F', or
    /// - U+0061 'a' ... U+0066 'f'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(uppercase_a.is_ascii_hexdigit());
    /// assert!(!uppercase_g.is_ascii_hexdigit());
    /// assert!(a.is_ascii_hexdigit());
    /// assert!(!g.is_ascii_hexdigit());
    /// assert!(zero.is_ascii_hexdigit());
    /// assert!(!percent.is_ascii_hexdigit());
    /// assert!(!space.is_ascii_hexdigit());
    /// assert!(!lf.is_ascii_hexdigit());
    /// assert!(!esc.is_ascii_hexdigit());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_hexdigit(&self) -> bool {
        if *self >= 0x80 { return false }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            D | Lx | Ux => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII punctuation character:
    ///
    /// - U+0021 ... U+002F `! " # $ % & ' ( ) * + , - . /`, or
    /// - U+003A ... U+0040 `: ; < = > ? @`, or
    /// - U+005B ... U+0060 ``[ \ ] ^ _ ` ``, or
    /// - U+007B ... U+007E `{ | } ~`
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(!uppercase_a.is_ascii_punctuation());
    /// assert!(!uppercase_g.is_ascii_punctuation());
    /// assert!(!a.is_ascii_punctuation());
    /// assert!(!g.is_ascii_punctuation());
    /// assert!(!zero.is_ascii_punctuation());
    /// assert!(percent.is_ascii_punctuation());
    /// assert!(!space.is_ascii_punctuation());
    /// assert!(!lf.is_ascii_punctuation());
    /// assert!(!esc.is_ascii_punctuation());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_punctuation(&self) -> bool {
        if *self >= 0x80 { return false }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            P => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII graphic character:
    /// U+0021 '!' ... U+007E '~'.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(uppercase_a.is_ascii_graphic());
    /// assert!(uppercase_g.is_ascii_graphic());
    /// assert!(a.is_ascii_graphic());
    /// assert!(g.is_ascii_graphic());
    /// assert!(zero.is_ascii_graphic());
    /// assert!(percent.is_ascii_graphic());
    /// assert!(!space.is_ascii_graphic());
    /// assert!(!lf.is_ascii_graphic());
    /// assert!(!esc.is_ascii_graphic());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_graphic(&self) -> bool {
        if *self >= 0x80 { return false; }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            Ux | U | Lx | L | D | P => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII whitespace character:
    /// U+0020 SPACE, U+0009 HORIZONTAL TAB, U+000A LINE FEED,
    /// U+000C FORM FEED, or U+000D CARRIAGE RETURN.
    ///
    /// Rust uses the WhatWG Infra Standard's [definition of ASCII
    /// whitespace][infra-aw]. There are several other definitions in
    /// wide use. For instance, [the POSIX locale][pct] includes
    /// U+000B VERTICAL TAB as well as all the above characters,
    /// but—from the very same specification—[the default rule for
    /// "field splitting" in the Bourne shell][bfs] considers *only*
    /// SPACE, HORIZONTAL TAB, and LINE FEED as whitespace.
    ///
    /// If you are writing a program that will process an existing
    /// file format, check what that format's definition of whitespace is
    /// before using this function.
    ///
    /// [infra-aw]: https://infra.spec.whatwg.org/#ascii-whitespace
    /// [pct]: http://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap07.html#tag_07_03_01
    /// [bfs]: http://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_06_05
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(!uppercase_a.is_ascii_whitespace());
    /// assert!(!uppercase_g.is_ascii_whitespace());
    /// assert!(!a.is_ascii_whitespace());
    /// assert!(!g.is_ascii_whitespace());
    /// assert!(!zero.is_ascii_whitespace());
    /// assert!(!percent.is_ascii_whitespace());
    /// assert!(space.is_ascii_whitespace());
    /// assert!(lf.is_ascii_whitespace());
    /// assert!(!esc.is_ascii_whitespace());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_whitespace(&self) -> bool {
        if *self >= 0x80 { return false; }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            Cw | W => true,
            _ => false
        }
    }

    /// Checks if the value is an ASCII control character:
    /// U+0000 NUL ... U+001F UNIT SEPARATOR, or U+007F DELETE.
    /// Note that most ASCII whitespace characters are control
    /// characters, but SPACE is not.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ascii_ctype)]
    ///
    /// let uppercase_a = b'A';
    /// let uppercase_g = b'G';
    /// let a = b'a';
    /// let g = b'g';
    /// let zero = b'0';
    /// let percent = b'%';
    /// let space = b' ';
    /// let lf = b'\n';
    /// let esc = 0x1b_u8;
    ///
    /// assert!(!uppercase_a.is_ascii_control());
    /// assert!(!uppercase_g.is_ascii_control());
    /// assert!(!a.is_ascii_control());
    /// assert!(!g.is_ascii_control());
    /// assert!(!zero.is_ascii_control());
    /// assert!(!percent.is_ascii_control());
    /// assert!(!space.is_ascii_control());
    /// assert!(lf.is_ascii_control());
    /// assert!(esc.is_ascii_control());
    /// ```
    #[stable(feature = "ascii_ctype_on_intrinsics", since = "1.24.0")]
    #[inline]
    pub fn is_ascii_control(&self) -> bool {
        if *self >= 0x80 { return false; }
        match ASCII_CHARACTER_CLASS[*self as usize] {
            C | Cw => true,
            _ => false
        }
    }
}

#[lang = "u16"]
impl u16 {
    uint_impl! { u16, u16, 16, 65535, "", "" }
}

#[lang = "u32"]
impl u32 {
    uint_impl! { u32, u32, 32, 4294967295, "", "" }
}

#[lang = "u64"]
impl u64 {
    uint_impl! { u64, u64, 64, 18446744073709551615, "", "" }
}

#[lang = "u128"]
impl u128 {
    uint_impl! { u128, u128, 128, 340282366920938463463374607431768211455, "", "" }
}

#[cfg(target_pointer_width = "16")]
#[lang = "usize"]
impl usize {
    uint_impl! { usize, u16, 16, 65536, "", "" }
}
#[cfg(target_pointer_width = "32")]
#[lang = "usize"]
impl usize {
    uint_impl! { usize, u32, 32, 4294967295, "", "" }
}

#[cfg(target_pointer_width = "64")]
#[lang = "usize"]
impl usize {
    uint_impl! { usize, u64, 64, 18446744073709551615, "", "" }
}

/// A classification of floating point numbers.
///
/// This `enum` is used as the return type for [`f32::classify`] and [`f64::classify`]. See
/// their documentation for more.
///
/// [`f32::classify`]: ../../std/primitive.f32.html#method.classify
/// [`f64::classify`]: ../../std/primitive.f64.html#method.classify
///
/// # Examples
///
/// ```
/// use std::num::FpCategory;
/// use std::f32;
///
/// let num = 12.4_f32;
/// let inf = f32::INFINITY;
/// let zero = 0f32;
/// let sub: f32 = 1.1754942e-38;
/// let nan = f32::NAN;
///
/// assert_eq!(num.classify(), FpCategory::Normal);
/// assert_eq!(inf.classify(), FpCategory::Infinite);
/// assert_eq!(zero.classify(), FpCategory::Zero);
/// assert_eq!(nan.classify(), FpCategory::Nan);
/// assert_eq!(sub.classify(), FpCategory::Subnormal);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[stable(feature = "rust1", since = "1.0.0")]
pub enum FpCategory {
    /// "Not a Number", often obtained by dividing by zero.
    #[stable(feature = "rust1", since = "1.0.0")]
    Nan,

    /// Positive or negative infinity.
    #[stable(feature = "rust1", since = "1.0.0")]
    Infinite,

    /// Positive or negative zero.
    #[stable(feature = "rust1", since = "1.0.0")]
    Zero,

    /// De-normalized floating point representation (less precise than `Normal`).
    #[stable(feature = "rust1", since = "1.0.0")]
    Subnormal,

    /// A regular floating point number.
    #[stable(feature = "rust1", since = "1.0.0")]
    Normal,
}

macro_rules! from_str_radix_int_impl {
    ($($t:ty)*) => {$(
        #[stable(feature = "rust1", since = "1.0.0")]
        impl FromStr for $t {
            type Err = ParseIntError;
            fn from_str(src: &str) -> Result<Self, ParseIntError> {
                from_str_radix(src, 10)
            }
        }
    )*}
}
from_str_radix_int_impl! { isize i8 i16 i32 i64 i128 usize u8 u16 u32 u64 u128 }

/// The error type returned when a checked integral type conversion fails.
#[unstable(feature = "try_from", issue = "33417")]
#[derive(Debug, Copy, Clone)]
pub struct TryFromIntError(());

impl TryFromIntError {
    #[unstable(feature = "int_error_internals",
               reason = "available through Error trait and this method should \
                         not be exposed publicly",
               issue = "0")]
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        "out of range integral type conversion attempted"
    }
}

#[unstable(feature = "try_from", issue = "33417")]
impl fmt::Display for TryFromIntError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.__description().fmt(fmt)
    }
}

#[unstable(feature = "try_from", issue = "33417")]
impl From<!> for TryFromIntError {
    fn from(never: !) -> TryFromIntError {
        never
    }
}

// only negative bounds
macro_rules! try_from_lower_bounded {
    ($source:ty, $($target:ty),*) => {$(
        #[unstable(feature = "try_from", issue = "33417")]
        impl TryFrom<$source> for $target {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(u: $source) -> Result<$target, TryFromIntError> {
                if u >= 0 {
                    Ok(u as $target)
                } else {
                    Err(TryFromIntError(()))
                }
            }
        }
    )*}
}

// unsigned to signed (only positive bound)
macro_rules! try_from_upper_bounded {
    ($source:ty, $($target:ty),*) => {$(
        #[unstable(feature = "try_from", issue = "33417")]
        impl TryFrom<$source> for $target {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(u: $source) -> Result<$target, TryFromIntError> {
                if u > (<$target>::max_value() as $source) {
                    Err(TryFromIntError(()))
                } else {
                    Ok(u as $target)
                }
            }
        }
    )*}
}

// all other cases
macro_rules! try_from_both_bounded {
    ($source:ty, $($target:ty),*) => {$(
        #[unstable(feature = "try_from", issue = "33417")]
        impl TryFrom<$source> for $target {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(u: $source) -> Result<$target, TryFromIntError> {
                let min = <$target>::min_value() as $source;
                let max = <$target>::max_value() as $source;
                if u < min || u > max {
                    Err(TryFromIntError(()))
                } else {
                    Ok(u as $target)
                }
            }
        }
    )*}
}

macro_rules! rev {
    ($mac:ident, $source:ty, $($target:ty),*) => {$(
        $mac!($target, $source);
    )*}
}

/// intra-sign conversions
try_from_upper_bounded!(u16, u8);
try_from_upper_bounded!(u32, u16, u8);
try_from_upper_bounded!(u64, u32, u16, u8);
try_from_upper_bounded!(u128, u64, u32, u16, u8);

try_from_both_bounded!(i16, i8);
try_from_both_bounded!(i32, i16, i8);
try_from_both_bounded!(i64, i32, i16, i8);
try_from_both_bounded!(i128, i64, i32, i16, i8);

// unsigned-to-signed
try_from_upper_bounded!(u8, i8);
try_from_upper_bounded!(u16, i8, i16);
try_from_upper_bounded!(u32, i8, i16, i32);
try_from_upper_bounded!(u64, i8, i16, i32, i64);
try_from_upper_bounded!(u128, i8, i16, i32, i64, i128);

// signed-to-unsigned
try_from_lower_bounded!(i8, u8, u16, u32, u64, u128);
try_from_lower_bounded!(i16, u16, u32, u64, u128);
try_from_lower_bounded!(i32, u32, u64, u128);
try_from_lower_bounded!(i64, u64, u128);
try_from_lower_bounded!(i128, u128);
try_from_both_bounded!(i16, u8);
try_from_both_bounded!(i32, u16, u8);
try_from_both_bounded!(i64, u32, u16, u8);
try_from_both_bounded!(i128, u64, u32, u16, u8);

// usize/isize
try_from_upper_bounded!(usize, isize);
try_from_lower_bounded!(isize, usize);

try_from_upper_bounded!(usize, u8);
try_from_upper_bounded!(usize, i8, i16);
try_from_both_bounded!(isize, u8);
try_from_both_bounded!(isize, i8);

#[cfg(target_pointer_width = "16")]
mod ptr_try_from_impls {
    use super::TryFromIntError;
    use convert::TryFrom;

    // Fallible across platfoms, only implementation differs
    try_from_lower_bounded!(isize, u16, u32, u64, u128);
    rev!(try_from_lower_bounded, usize, i8, i16);
    rev!(try_from_both_bounded, usize, i32, i64, i128);
}

#[cfg(target_pointer_width = "32")]
mod ptr_try_from_impls {
    use super::TryFromIntError;
    use convert::TryFrom;

    // Fallible across platfoms, only implementation differs
    try_from_both_bounded!(isize, u16);
    try_from_lower_bounded!(isize, u32, u64, u128);
    rev!(try_from_lower_bounded, usize, i8, i16, i32);
    rev!(try_from_both_bounded, usize, i64, i128);
}

#[cfg(target_pointer_width = "64")]
mod ptr_try_from_impls {
    use super::TryFromIntError;
    use convert::TryFrom;

    // Fallible across platfoms, only implementation differs
    try_from_both_bounded!(isize, u16, u32);
    try_from_lower_bounded!(isize, u64, u128);
    rev!(try_from_lower_bounded, usize, i8, i16, i32, i64);
    rev!(try_from_both_bounded, usize, i128);
}

#[doc(hidden)]
trait FromStrRadixHelper: PartialOrd + Copy {
    fn min_value() -> Self;
    fn max_value() -> Self;
    fn from_u32(u: u32) -> Self;
    fn checked_mul(&self, other: u32) -> Option<Self>;
    fn checked_sub(&self, other: u32) -> Option<Self>;
    fn checked_add(&self, other: u32) -> Option<Self>;
}

macro_rules! doit {
    ($($t:ty)*) => ($(impl FromStrRadixHelper for $t {
        #[inline]
        fn min_value() -> Self { Self::min_value() }
        #[inline]
        fn max_value() -> Self { Self::max_value() }
        #[inline]
        fn from_u32(u: u32) -> Self { u as Self }
        #[inline]
        fn checked_mul(&self, other: u32) -> Option<Self> {
            Self::checked_mul(*self, other as Self)
        }
        #[inline]
        fn checked_sub(&self, other: u32) -> Option<Self> {
            Self::checked_sub(*self, other as Self)
        }
        #[inline]
        fn checked_add(&self, other: u32) -> Option<Self> {
            Self::checked_add(*self, other as Self)
        }
    })*)
}
doit! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

fn from_str_radix<T: FromStrRadixHelper>(src: &str, radix: u32) -> Result<T, ParseIntError> {
    use self::IntErrorKind::*;
    use self::ParseIntError as PIE;

    assert!(radix >= 2 && radix <= 36,
           "from_str_radix_int: must lie in the range `[2, 36]` - found {}",
           radix);

    if src.is_empty() {
        return Err(PIE { kind: Empty });
    }

    let is_signed_ty = T::from_u32(0) > T::min_value();

    // all valid digits are ascii, so we will just iterate over the utf8 bytes
    // and cast them to chars. .to_digit() will safely return None for anything
    // other than a valid ascii digit for the given radix, including the first-byte
    // of multi-byte sequences
    let src = src.as_bytes();

    let (is_positive, digits) = match src[0] {
        b'+' => (true, &src[1..]),
        b'-' if is_signed_ty => (false, &src[1..]),
        _ => (true, src),
    };

    if digits.is_empty() {
        return Err(PIE { kind: Empty });
    }

    let mut result = T::from_u32(0);
    if is_positive {
        // The number is positive
        for &c in digits {
            let x = match (c as char).to_digit(radix) {
                Some(x) => x,
                None => return Err(PIE { kind: InvalidDigit }),
            };
            result = match result.checked_mul(radix) {
                Some(result) => result,
                None => return Err(PIE { kind: Overflow }),
            };
            result = match result.checked_add(x) {
                Some(result) => result,
                None => return Err(PIE { kind: Overflow }),
            };
        }
    } else {
        // The number is negative
        for &c in digits {
            let x = match (c as char).to_digit(radix) {
                Some(x) => x,
                None => return Err(PIE { kind: InvalidDigit }),
            };
            result = match result.checked_mul(radix) {
                Some(result) => result,
                None => return Err(PIE { kind: Underflow }),
            };
            result = match result.checked_sub(x) {
                Some(result) => result,
                None => return Err(PIE { kind: Underflow }),
            };
        }
    }
    Ok(result)
}

/// An error which can be returned when parsing an integer.
///
/// This error is used as the error type for the `from_str_radix()` functions
/// on the primitive integer types, such as [`i8::from_str_radix`].
///
/// # Potential causes
///
/// Among other causes, `ParseIntError` can be thrown because of leading or trailing whitespace
/// in the string e.g. when it is obtained from the standard input.
/// Using the [`str.trim()`] method ensures that no whitespace remains before parsing.
///
/// [`str.trim()`]: ../../std/primitive.str.html#method.trim
/// [`i8::from_str_radix`]: ../../std/primitive.i8.html#method.from_str_radix
#[derive(Debug, Clone, PartialEq, Eq)]
#[stable(feature = "rust1", since = "1.0.0")]
pub struct ParseIntError {
    kind: IntErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IntErrorKind {
    Empty,
    InvalidDigit,
    Overflow,
    Underflow,
}

impl ParseIntError {
    #[unstable(feature = "int_error_internals",
               reason = "available through Error trait and this method should \
                         not be exposed publicly",
               issue = "0")]
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        match self.kind {
            IntErrorKind::Empty => "cannot parse integer from empty string",
            IntErrorKind::InvalidDigit => "invalid digit found in string",
            IntErrorKind::Overflow => "number too large to fit in target type",
            IntErrorKind::Underflow => "number too small to fit in target type",
        }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl fmt::Display for ParseIntError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.__description().fmt(f)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
pub use num::dec2flt::ParseFloatError;

// Conversion traits for primitive integer and float types
// Conversions T -> T are covered by a blanket impl and therefore excluded
// Some conversions from and to usize/isize are not implemented due to portability concerns
macro_rules! impl_from {
    ($Small: ty, $Large: ty, #[$attr:meta], $doc: expr) => {
        #[$attr]
        #[doc = $doc]
        impl From<$Small> for $Large {
            #[inline]
            fn from(small: $Small) -> $Large {
                small as $Large
            }
        }
    };
    ($Small: ty, $Large: ty, #[$attr:meta]) => {
        impl_from!($Small,
                   $Large,
                   #[$attr],
                   concat!("Converts `",
                           stringify!($Small),
                           "` to `",
                           stringify!($Large),
                           "` losslessly."));
    }
}

macro_rules! impl_from_bool {
    ($target: ty, #[$attr:meta]) => {
        impl_from!(bool, $target, #[$attr], concat!("Converts a `bool` to a `",
            stringify!($target), "`. The resulting value is `0` for `false` and `1` for `true`
values.

# Examples

```
assert_eq!(", stringify!($target), "::from(true), 1);
assert_eq!(", stringify!($target), "::from(false), 0);
```"));
    };
}

// Bool -> Any
impl_from_bool! { u8, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { u16, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { u32, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { u64, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { u128, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { usize, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { i8, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { i16, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { i32, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { i64, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { i128, #[stable(feature = "from_bool", since = "1.28.0")] }
impl_from_bool! { isize, #[stable(feature = "from_bool", since = "1.28.0")] }

// Unsigned -> Unsigned
impl_from! { u8, u16, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u8, u32, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u8, u64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u8, u128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { u8, usize, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u16, u32, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u16, u64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u16, u128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { u32, u64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u32, u128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { u64, u128, #[stable(feature = "i128", since = "1.26.0")] }

// Signed -> Signed
impl_from! { i8, i16, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i8, i32, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i8, i64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i8, i128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { i8, isize, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i16, i32, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i16, i64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i16, i128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { i32, i64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { i32, i128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { i64, i128, #[stable(feature = "i128", since = "1.26.0")] }

// Unsigned -> Signed
impl_from! { u8, i16, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u8, i32, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u8, i64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u8, i128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { u16, i32, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u16, i64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u16, i128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { u32, i64, #[stable(feature = "lossless_int_conv", since = "1.5.0")] }
impl_from! { u32, i128, #[stable(feature = "i128", since = "1.26.0")] }
impl_from! { u64, i128, #[stable(feature = "i128", since = "1.26.0")] }

// The C99 standard defines bounds on INTPTR_MIN, INTPTR_MAX, and UINTPTR_MAX
// which imply that pointer-sized integers must be at least 16 bits:
// https://port70.net/~nsz/c/c99/n1256.html#7.18.2.4
impl_from! { u16, usize, #[stable(feature = "lossless_iusize_conv", since = "1.26.0")] }
impl_from! { u8, isize, #[stable(feature = "lossless_iusize_conv", since = "1.26.0")] }
impl_from! { i16, isize, #[stable(feature = "lossless_iusize_conv", since = "1.26.0")] }

// RISC-V defines the possibility of a 128-bit address space (RV128).

// CHERI proposes 256-bit “capabilities”. Unclear if this would be relevant to usize/isize.
// https://www.cl.cam.ac.uk/research/security/ctsrd/pdfs/20171017a-cheri-poster.pdf
// http://www.csl.sri.com/users/neumann/2012resolve-cheri.pdf


// Note: integers can only be represented with full precision in a float if
// they fit in the significand, which is 24 bits in f32 and 53 bits in f64.
// Lossy float conversions are not implemented at this time.

// Signed -> Float
impl_from! { i8, f32, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { i8, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { i16, f32, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { i16, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { i32, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }

// Unsigned -> Float
impl_from! { u8, f32, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { u8, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { u16, f32, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { u16, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }
impl_from! { u32, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }

// Float -> Float
impl_from! { f32, f64, #[stable(feature = "lossless_float_conv", since = "1.6.0")] }

static ASCII_LOWERCASE_MAP: [u8; 256] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    b' ', b'!', b'"', b'#', b'$', b'%', b'&', b'\'',
    b'(', b')', b'*', b'+', b',', b'-', b'.', b'/',
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'8', b'9', b':', b';', b'<', b'=', b'>', b'?',
    b'@',

          b'a', b'b', b'c', b'd', b'e', b'f', b'g',
    b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o',
    b'p', b'q', b'r', b's', b't', b'u', b'v', b'w',
    b'x', b'y', b'z',

                      b'[', b'\\', b']', b'^', b'_',
    b'`', b'a', b'b', b'c', b'd', b'e', b'f', b'g',
    b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o',
    b'p', b'q', b'r', b's', b't', b'u', b'v', b'w',
    b'x', b'y', b'z', b'{', b'|', b'}', b'~', 0x7f,
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f,
    0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
    0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f,
    0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf,
    0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7,
    0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf,
    0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf,
    0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7,
    0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf,
    0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
    0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef,
    0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
    0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
];

static ASCII_UPPERCASE_MAP: [u8; 256] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    b' ', b'!', b'"', b'#', b'$', b'%', b'&', b'\'',
    b'(', b')', b'*', b'+', b',', b'-', b'.', b'/',
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'8', b'9', b':', b';', b'<', b'=', b'>', b'?',
    b'@', b'A', b'B', b'C', b'D', b'E', b'F', b'G',
    b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O',
    b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W',
    b'X', b'Y', b'Z', b'[', b'\\', b']', b'^', b'_',
    b'`',

          b'A', b'B', b'C', b'D', b'E', b'F', b'G',
    b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O',
    b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W',
    b'X', b'Y', b'Z',

                      b'{', b'|', b'}', b'~', 0x7f,
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f,
    0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
    0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f,
    0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf,
    0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7,
    0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf,
    0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf,
    0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7,
    0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf,
    0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
    0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef,
    0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
    0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
];

enum AsciiCharacterClass {
    C,  // control
    Cw, // control whitespace
    W,  // whitespace
    D,  // digit
    L,  // lowercase
    Lx, // lowercase hex digit
    U,  // uppercase
    Ux, // uppercase hex digit
    P,  // punctuation
}
use self::AsciiCharacterClass::*;

static ASCII_CHARACTER_CLASS: [AsciiCharacterClass; 128] = [
//  _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 _a _b _c _d _e _f
    C, C, C, C, C, C, C, C, C, Cw,Cw,C, Cw,Cw,C, C, // 0_
    C, C, C, C, C, C, C, C, C, C, C, C, C, C, C, C, // 1_
    W, P, P, P, P, P, P, P, P, P, P, P, P, P, P, P, // 2_
    D, D, D, D, D, D, D, D, D, D, P, P, P, P, P, P, // 3_
    P, Ux,Ux,Ux,Ux,Ux,Ux,U, U, U, U, U, U, U, U, U, // 4_
    U, U, U, U, U, U, U, U, U, U, U, P, P, P, P, P, // 5_
    P, Lx,Lx,Lx,Lx,Lx,Lx,L, L, L, L, L, L, L, L, L, // 6_
    L, L, L, L, L, L, L, L, L, L, L, P, P, P, P, C, // 7_
];
