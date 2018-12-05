extern crate vlq;

use std::i64;
use std::u32;
use vlq::{decode, encode, Error};

fn decode_tester_ok(input: &[u8], expect: i64) {
    println!("decoding '{}'", String::from_utf8_lossy(input));
    let mut input = input.iter().cloned();
    match decode(&mut input) {
        Ok(x) => {
            assert_eq!(x, expect);
            assert!(input.next().is_none());
        }
        err => panic!("failed to decode: {:?}", err),
    }
}

#[test]
fn test_decode() {
    decode_tester_ok("A".as_bytes(), 0);
    decode_tester_ok("B".as_bytes(), 0);
    decode_tester_ok("C".as_bytes(), 1);
    decode_tester_ok("D".as_bytes(), -1);
}

fn roundtrip_ok(val: i64) {
    println!("----------------------------------------");
    println!("encoding {}", val);
    println!("         {:064b}", val);
    let mut buf = Vec::<u8>::new();
    match encode(val, &mut buf) {
        Ok(()) => assert!(buf.len() > 0),
        err => panic!("failed to encode: {:?}", err),
    }
    decode_tester_ok(&buf, val);
}

#[test]
fn test_roundtrip_mids() {
    for val in -512..513 {
        roundtrip_ok(val);
        roundtrip_ok(u32::MAX as i64 + val);
        roundtrip_ok(-(u32::MAX as i64) + val);
    }
}

#[test]
fn test_roundtrip_mins() {
    for val in i64::MIN + 1..i64::MIN + 1024 {
        roundtrip_ok(val);
    }
}

#[test]
fn test_roundtrip_maxs() {
    for val in i64::MAX - 1024..i64::MAX {
        roundtrip_ok(val);
    }
    roundtrip_ok(i64::MAX);
}

#[test]
fn test_wrapping() {
    let inputs = &[
        // `i64::MIN`
        &b"hgggggggggggI"[..],

        &b"////////////////////////////A"[..],
        &b"////////////P"[..],
    ];

    for input in inputs {
        match decode(&mut input.iter().cloned()) {
            Err(Error::Overflow) => {},
            Ok(val) => {
                println!("WHAT?? {} {:x}", val, i64::max_value());
            },
            _ => assert!(false),
        }
    }
}
