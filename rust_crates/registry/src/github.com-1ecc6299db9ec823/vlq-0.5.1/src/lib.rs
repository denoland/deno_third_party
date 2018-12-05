//! An implementation of the Base64-encoded
//! [VLQ](https://en.wikipedia.org/wiki/Variable-length_quantity)
//! encoding.  Note that there are several variants of VLQ.  This only
//! implements the variant used by [source
//! maps](https://github.com/mozilla/source-map).

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

use std::io;
use std::i64;

// Some constants defined by the spec.
const SHIFT: u8 = 5;
const MASK: u8 = (1 << SHIFT) - 1;
const CONTINUED: u8 = 1 << SHIFT;

/// An error that occurred while decoding.
#[derive(Debug)]
pub enum Error {
    /// Unexpectedly hit EOF.
    UnexpectedEof,
    /// The input contained an invalid byte.
    InvalidBase64(u8),
    /// The input encoded a number that didn't fit into i64.
    Overflow,
}

/// The result of decoding.
pub type Result<T> = std::result::Result<T, Error>;

// Decode a single base64 digit.
fn decode64(input: u8) -> Result<u8> {
    match input {
        b'A'...b'Z' => Ok(input - b'A'),
        b'a'...b'z' => Ok(input - b'a' + 26),
        b'0'...b'9' => Ok(input - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(Error::InvalidBase64(input)),
    }
}

/// Decode a single VLQ value from the input, returning the value.
///
/// # Range
///
/// Supports all numbers that can be represented by a sign bit and a 63 bit
/// absolute value: `[-(2^63 - 1), 2^63 - 1]`.
///
/// Note that `i64::MIN = -(2^63)` cannot be represented in that form, and this
/// function will return `Error::Overflowed` when attempting to decode it.
pub fn decode<B>(input: &mut B) -> Result<i64>
where
    B: Iterator<Item = u8>,
{
    let mut accum: u64 = 0;
    let mut shift = 0;

    let mut keep_going = true;
    while keep_going {
        let byte = input.next().ok_or(Error::UnexpectedEof)?;
        let digit = decode64(byte)?;
        keep_going = (digit & CONTINUED) != 0;

        let digit_value = ((digit & MASK) as u64)
            .checked_shl(shift as u32)
            .ok_or(Error::Overflow)?;

        accum = accum.checked_add(digit_value).ok_or(Error::Overflow)?;
        shift += SHIFT;
    }

    let abs_value = accum / 2;
    if abs_value > (i64::MAX as u64) {
        return Err(Error::Overflow);
    }

    // The low bit holds the sign.
    if (accum & 1) != 0 {
        Ok(-(abs_value as i64))
    } else {
        Ok(abs_value as i64)
    }
}

// Encode a single base64 digit.
fn encode64(value: u8) -> u8 {
    debug_assert!(value < 64);
    if value < 26 {
        value + b'A'
    } else if value < 52 {
        value - 26 + b'a'
    } else if value < 62 {
        value - 52 + b'0'
    } else if value == 62 {
        b'+'
    } else {
        assert!(value == 63);
        b'/'
    }
}

/// Encode a value as Base64 VLQ, sending it to the writer.
pub fn encode<W>(value: i64, output: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    let signed = value < 0;
    let mut value = (value.wrapping_abs() as u64) << 1;
    if signed {
        if value == 0 {
            // Wrapped.
            value = (i64::MAX as u64) + 1;
        }

        value |= 1;
    }
    loop {
        let mut digit = value as u8 & MASK;
        value >>= SHIFT;
        if value > 0 {
            digit |= CONTINUED;
        }
        let bytes = [encode64(digit)];
        output.write_all(&bytes[..])?;
        if value == 0 {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_simple_roundtrip() {
        for val in 0..64 {
            // Don't crash, and decoding is an identity.
            assert_eq!(val, super::decode64(super::encode64(val)).unwrap());
        }
    }
}
