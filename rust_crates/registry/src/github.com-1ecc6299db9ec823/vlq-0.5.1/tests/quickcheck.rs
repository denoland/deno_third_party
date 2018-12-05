#[macro_use]
extern crate quickcheck;
extern crate vlq;

use std::i64;
use vlq::{decode, encode};

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

quickcheck! {
    fn parse_check(inputs: Vec<u8>) -> () {
        let mut coded = inputs.into_iter().map(|x| {
            encode64(x & 63)
        });
        let _ = decode(&mut coded);
    }

    fn roundtrip(x: i64) -> bool {
        // The single `i64` value that we cannot round trip.
        if x == i64::MIN {
            return true;
        }

        let mut buf = vec![];
        encode(x, &mut buf).unwrap();
        decode(&mut buf.iter().cloned()).unwrap() == x
    }
}
