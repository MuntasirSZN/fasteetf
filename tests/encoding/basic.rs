use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

// ── Integer encoding ───────────────────────────────────────────────────────

#[test]
fn test_encode_small_int() {
    // 42 fits in 0-255, should use SMALL_INTEGER_EXT.
    let encoded = encode_ok(&Term::Int(42));
    assert_eq!(encoded, &[131, 97, 42]);

    // 0 is the lower bound of SMALL_INTEGER_EXT.
    let encoded = encode_ok(&Term::Int(0));
    assert_eq!(encoded, &[131, 97, 0]);

    // 255 is the upper bound of SMALL_INTEGER_EXT.
    let encoded = encode_ok(&Term::Int(255));
    assert_eq!(encoded, &[131, 97, 255]);
}

#[test]
fn test_encode_large_int() {
    // 256 must use INTEGER_EXT.
    let encoded = encode_ok(&Term::Int(256));
    assert_eq!(encoded, &[131, 98, 0, 0, 1, 0]);

    // Negative numbers use INTEGER_EXT.
    let encoded = encode_ok(&Term::Int(-1));
    assert_eq!(encoded, &[131, 98, 255, 255, 255, 255]);

    // Max i32.
    let encoded = encode_ok(&Term::Int(i32::MAX));
    assert_eq!(encoded, &[131, 98, 127, 255, 255, 255]);

    // Min i32.
    let encoded = encode_ok(&Term::Int(i32::MIN));
    assert_eq!(encoded, &[131, 98, 128, 0, 0, 0]);
}

// ── Float encoding ─────────────────────────────────────────────────────────

#[test]
fn test_encode_float() {
    let encoded = encode_ok(&Term::Float(42.0));
    assert_eq!(encoded.len(), 10); // magic + NEW_FLOAT_EXT + 8 bytes
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 70); // NEW_FLOAT_EXT

    // Roundtrip.
    with_parse(&encoded, |parsed| {
        assert!(matches!(parsed, Term::Float(v) if v == 42.0));
    });
}

#[test]
fn test_encode_float_nan() {
    let encoded = encode_ok(&Term::Float(f64::NAN));
    with_parse(&encoded, |parsed| {
        assert!(matches!(parsed, Term::Float(v) if v.is_nan()));
    });
}

// ── Atom encoding ──────────────────────────────────────────────────────────

#[test]
fn test_encode_small_atom() {
    let a = unsafe { AtomUtf8::from_bytes_unchecked(b"hello") };
    let encoded = encode_ok(&Term::Atom(a));
    // SMALL_ATOM_UTF8_EXT (119) + len(5) + "hello"
    assert_eq!(encoded, &[131, 119, 5, b'h', b'e', b'l', b'l', b'o']);
}

#[test]
fn test_encode_empty_atom() {
    let a = unsafe { AtomUtf8::from_bytes_unchecked(b"") };
    let encoded = encode_ok(&Term::Atom(a));
    assert_eq!(encoded, &[131, 119, 0]);
}

#[test]
fn test_encode_large_atom() {
    // 300-byte atom → ATOM_UTF8_EXT (118) + 2-byte length.
    let bytes: Vec<u8> = (0usize..300).map(|i| (i % 26) as u8 + b'a').collect();
    let a = unsafe { AtomUtf8::from_bytes_unchecked(&bytes) };
    let encoded = encode_ok(&Term::Atom(a));
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 118); // ATOM_UTF8_EXT
    let len = u16::from_be_bytes([encoded[2], encoded[3]]);
    assert_eq!(len, 300);
    assert_eq!(&encoded[4..], &bytes[..]);
}

#[test]
fn test_encode_atom_roundtrip() {
    let bytes = b"erlang";
    let a = unsafe { AtomUtf8::from_bytes_unchecked(bytes) };
    let encoded = encode_ok(&Term::Atom(a));
    with_parse(&encoded, |parsed| match parsed {
        Term::Atom(a2) => assert_eq!(a2.as_str(), Ok("erlang")),
        _ => panic!("expected Atom"),
    });
}

// ── Binary encoding ────────────────────────────────────────────────────────

#[test]
fn test_encode_binary() {
    let data = b"\x00\x01\x02\x03";
    let encoded = encode_ok(&Term::Binary(&data[..]));
    assert_eq!(encoded, &[131, 109, 0, 0, 0, 4, 0, 1, 2, 3]);
}

#[test]
fn test_encode_empty_binary() {
    let encoded = encode_ok(&Term::Binary(&[]));
    assert_eq!(encoded, &[131, 109, 0, 0, 0, 0]);
}

// ── BitBinary encoding ─────────────────────────────────────────────────────

#[test]
fn test_encode_bit_binary() {
    let encoded = encode_ok(&Term::BitBinary {
        bits: 4,
        data: &[0xAB],
    });
    assert_eq!(encoded, &[131, 77, 0, 0, 0, 1, 4, 0xAB]);
}

// ── Tuple encoding ─────────────────────────────────────────────────────────

#[test]
fn test_encode_empty_tuple() {
    let encoded = encode_ok(&Term::Tuple(&[]));
    assert_eq!(encoded, &[131, 104, 0]);
}

#[test]
fn test_encode_small_tuple() {
    let terms = [Term::Int(1), Term::Int(2), Term::Int(3)];
    let encoded = encode_ok(&Term::Tuple(&terms));
    assert_eq!(encoded, &[131, 104, 3, 97, 1, 97, 2, 97, 3]);
}

#[test]
fn test_encode_large_tuple() {
    // 300-element tuple → LARGE_TUPLE_EXT
    let terms: Vec<Term<'_>> = (0usize..300).map(|i| Term::Int(i as i32)).collect();
    let encoded = encode_ok(&Term::Tuple(&terms));
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 105); // LARGE_TUPLE_EXT
    let arity = u32::from_be_bytes([encoded[2], encoded[3], encoded[4], encoded[5]]);
    assert_eq!(arity, 300);
}

// ── String encoding ────────────────────────────────────────────────────────

#[test]
fn test_encode_string_ext() {
    let encoded = encode_ok(&Term::String(b"abc"));
    assert_eq!(encoded, &[131, 107, 0, 3, b'a', b'b', b'c']);

    // Empty string uses STRING_EXT with length 0.
    let encoded = encode_ok(&Term::String(b""));
    assert_eq!(encoded, &[131, 107, 0, 0]);
}

// ── List encoding ──────────────────────────────────────────────────────────

#[test]
fn test_encode_empty_list() {
    let encoded = encode_ok(&Term::List(&[]));
    assert_eq!(encoded, &[131, 106]); // NIL_EXT
}

#[test]
fn test_encode_list() {
    let terms = [Term::Int(10), Term::Int(20)];
    let encoded = encode_ok(&Term::List(&terms));
    assert_eq!(encoded, &[131, 108, 0, 0, 0, 2, 97, 10, 97, 20, 106]);
}

#[test]
fn test_encode_improper_list() {
    // We need to construct an ImproperList, but our Term enum uses references.
    // Instead, parse an improper list first, then roundtrip.
    // ETF wire: [1 | 2] = LIST_EXT len=1, Int(1), Int(2) tail (no NIL).
    let input = b"\x83\x6c\x00\x00\x00\x01\x61\x01\x61\x02";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

// ── Improper-list encoding ─────────────────────────────────────────────────

#[test]
fn test_encode_improper_list_too_short() {
    // An improper list must have a tail; only the fully empty slice is
    // malformed. A single entry is a zero-element improper list
    // (LIST_EXT Len=0 + non-NIL tail), which the parser can produce and
    // which round-trips.
    let err = encode_to_vec(&Term::ImproperList(&[])).unwrap_err();
    assert!(matches!(err, EtfError::InvalidSize));
    let single = [Term::Int(1)];
    let encoded = encode_to_vec(&Term::ImproperList(&single)).unwrap();
    assert_eq!(encoded, [131, 108, 0, 0, 0, 0, 97, 1]);
}

#[test]
fn test_roundtrip_zero_element_improper_list() {
    // LIST_EXT Len=0 with a non-NIL tail parses to a one-entry
    // ImproperList and must encode back to identical bytes.
    let input = b"\x83\x6c\x00\x00\x00\x00\x77\x00"; // tail: empty atom
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

// ── Map encoding ───────────────────────────────────────────────────────────

#[test]
fn test_encode_empty_map() {
    let encoded = encode_ok(&Term::Map(&[]));
    assert_eq!(encoded, &[131, 116, 0, 0, 0, 0]);
}

#[test]
fn test_encode_map() {
    let pairs = [
        (
            Term::Int(1),
            Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"one") }),
        ),
        (
            Term::Int(2),
            Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"two") }),
        ),
    ];
    let encoded = encode_ok(&Term::Map(&pairs));
    with_parse(&encoded, |parsed| match parsed {
        Term::Map(p) => assert_eq!(p.len(), 2),
        _ => panic!("expected Map"),
    });
}

// ── Bignum encoding ────────────────────────────────────────────────────────

#[test]
fn test_encode_small_big() {
    // 256 in bignum format: digits=[0, 1] (little-endian), sign=0 (positive)
    let encoded = encode_ok(&Term::BigInt {
        sign: 0,
        digits: &[0, 1],
    });
    assert_eq!(encoded, &[131, 110, 2, 0, 0, 1]);
}

#[test]
fn test_encode_large_big() {
    // 300-digit bignum → LARGE_BIG_EXT
    let digits: Vec<u8> = (0usize..300).map(|_| 0xAB).collect();
    let encoded = encode_ok(&Term::BigInt {
        sign: 0,
        digits: &digits,
    });
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 111); // LARGE_BIG_EXT (auto-upgraded)
}

// ── Buffer overflow detection ──────────────────────────────────────────────

#[test]
fn test_encode_buffer_too_small() {
    let term = Term::Int(42);
    let mut buf = [0u8; 2]; // only enough for magic + tag, not the value
    let result = encode_to_buf(&term, &mut buf);
    assert!(result.is_err());
}
