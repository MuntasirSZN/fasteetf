// ─────────────────────────────────────────────────────────────────────────────
// Property-based round-trip tests using proptest.
//
// Each test generates random data, constructs an ETF byte sequence, parses it
// back, and asserts the result matches.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

mod common;
use common::*;
use fasteetf::*;

#[test]
fn test_proptest_roundtrip_small_int() {
    // SMALL_INTEGER_EXT is unsigned (0–255). Use u8 to avoid sign-extension issues.
    proptest::proptest!(|(val: u8)| {
        let encoded = [131, 97, val];
        with_parse(&encoded, |term| {
            assert!(matches!(term, Term::Int(v) if v == val as i32));
        });
    });
}

#[test]
fn test_proptest_roundtrip_int() {
    proptest::proptest!(|(val: i32)| {
        let mut encoded = vec![131, 98];
        encoded.extend_from_slice(&val.to_be_bytes());
        with_parse(&encoded, |term| {
            assert!(matches!(term, Term::Int(v) if v == val));
        });
    });
}

#[test]
fn test_proptest_roundtrip_float() {
    proptest::proptest!(|(val: f64)| {
        // Skip NaN because NaN != NaN
        if val.is_nan() { return Ok(()); }
        let mut encoded = vec![131, 70]; // NEW_FLOAT_EXT
        encoded.extend_from_slice(&val.to_be_bytes());
        with_parse(&encoded, |term| {
            assert!(matches!(term, Term::Float(v) if v == val));
        });
    });
}

#[test]
fn test_proptest_roundtrip_binary() {
    proptest::proptest!(|(data: Vec<u8>)| {
        if data.len() > 1024 { return Ok(()); } // keep it fast
        let mut encoded = vec![131, 109]; // BINARY_EXT
        encoded.extend_from_slice(&(data.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&data);
        with_parse(&encoded, |term| {
            assert!(matches!(term, Term::Binary(b) if b == &data[..]));
        });
    });
}

#[test]
fn test_proptest_roundtrip_atom() {
    proptest::proptest!(|(s: String)| {
        // Restrict to valid UTF-8 under 256 bytes
        if s.len() > 255 || s.is_empty() { return Ok(()); }
        let tag: u8 = if s.len() < 256 { 119 } else { 118 };
        let mut encoded = vec![131, tag];
        if tag == 119 {
            encoded.push(s.len() as u8);
        } else {
            encoded.extend_from_slice(&(s.len() as u16).to_be_bytes());
        }
        encoded.extend_from_slice(s.as_bytes());
        with_parse(&encoded, |term| match term {
            Term::Atom(a) => assert_eq!(a.as_str(), Ok(s.as_str())),
            _ => panic!("expected Atom"),
        });
    });
}
