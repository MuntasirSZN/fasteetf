// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for ETF encoding and comprehensive encode→parse roundtrips.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

mod common;
use common::*;
use fasteetf::*;

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
    let encoded = encode_ok(&Term::SmallBigInt {
        sign: 0,
        digits: &[0, 1],
    });
    assert_eq!(encoded, &[131, 110, 2, 0, 0, 1]);
}

#[test]
fn test_encode_large_big() {
    // 300-digit bignum → LARGE_BIG_EXT
    let digits: Vec<u8> = (0usize..300).map(|_| 0xAB).collect();
    let encoded = encode_ok(&Term::SmallBigInt {
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

// ── Comprehensive roundtrip: encode → parse ───────────────────────────────

#[test]
fn test_roundtrip_int() {
    for &v in &[0i32, 1, 127, 128, 255, 256, -1, i32::MIN, i32::MAX] {
        let encoded = encode_ok(&Term::Int(v));
        with_parse(&encoded, |parsed| {
            assert!(
                matches!(parsed, Term::Int(x) if x == v),
                "mismatch for {}",
                v
            );
        });
    }
}

#[test]
fn test_roundtrip_float() {
    for &v in &[0.0f64, 1.0, -1.0, 3.14159, 1.0e200, -2.5e-100] {
        let encoded = encode_ok(&Term::Float(v));
        with_parse(&encoded, |parsed| {
            assert!(
                matches!(parsed, Term::Float(x) if x == v),
                "mismatch for {}",
                v
            );
        });
    }
}

#[test]
fn test_roundtrip_list() {
    // Empty
    let encoded = encode_ok(&Term::List(&[]));
    with_parse(&encoded, |parsed| {
        assert!(matches!(parsed, Term::List(&[])));
    });

    // Non-empty
    let terms = [Term::Int(1), Term::Int(2), Term::Int(3)];
    let encoded = encode_ok(&Term::List(&terms));
    with_parse(&encoded, |parsed| match parsed {
        Term::List(l) => assert_eq!(l.len(), 3),
        _ => panic!("expected List"),
    });
}

#[test]
fn test_roundtrip_nested() {
    // Build a nested term via parse-then-encode.
    let input = b"\x83\x68\x02\x61\x01\x68\x02\x61\x02\x61\x03"; // {1, {2, 3}}
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_complex() {
    // Parse a complex term, encode it, and verify the encoded bytes match.
    let input = b"\x83\x74\x00\x00\x00\x02\x61\x01\x61\x02\x61\x03\x61\x04";
    // MAP #{1=>2, 3=>4}
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_pid() {
    // Build a PID term via parse (PID_EXT).
    // Wire: 103 Node=atom("node") ID=1 Serial=1 Creation=1
    let input = b"\x83\x67\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_new_pid() {
    // NEW_PID_EXT with 4-byte Creation.
    let input = b"\x83\x58\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_port_v4() {
    // V4_PORT_EXT.
    // Wire: 120 Node="node" ID=1 Creation=1
    let input = b"\x83\x78\x77\x04node\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_export() {
    // EXPORT_EXT: fun lists:map/2
    // Wire: 113 Module=atom("lists") Function=atom("map") Arity=int(2)
    let input = b"\x83\x71\x77\x05lists\x77\x03map\x61\x02";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_new_fun() {
    // NEW_FUN_EXT: fun with no free variables.
    // Wire: 112 Size(4) Arity(1) Uniq(16) Index(4) NumFree(4) Module OldIndex OldUniq Pid
    //
    // Breakdown of the payload bytes after Size:
    //   1  (Arity)
    // + 16 (Uniq)
    // + 4  (Index)
    // + 4  (NumFree)
    // + 3  (Module atom: tag 77 + len 1 + 'm')
    // + 2  (OldIndex: SMALL_INTEGER_EXT + 0)
    // + 2  (OldUniq: SMALL_INTEGER_EXT + 0)
    // + 16 (NEW_PID_EXT: tag 88 + atom "n" (3) + ID(4) + Serial(4) + Creation(4))
    //   = 48 bytes after Size
    // Size (includes the 4-byte Size field) = 4 + 48 = 52 = 0x34
    //
    // Full input (54 bytes):
    //   magic(1) + tag(1) + Size(4) + payload(48)
    let input = b"\x83\x70\x00\x00\x00\x34\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x77\x01\x6d\x61\x00\x61\x00\x58\x77\x01\x6e\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_record() {
    // RECORD_EXT: a native record.
    // Wire: 67 #Fields=1 Flags=1 Module=foo Name=bar FieldNames=[x] Values=[42]
    // We need to construct a valid record. Parse one.
    let input = b"\x83\x43\x00\x00\x00\x01\x01\x77\x03foo\x77\x03bar\x77\x01x\x61\x2a";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_bit_binary() {
    let input = b"\x83\x4d\x00\x00\x00\x02\x07\xab\xcd";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_reference() {
    // NEW_REFERENCE_EXT.
    // Wire: 114 Len=1 Node="node" Creation=1 ID=[1]
    let input = b"\x83\x72\x00\x01\x77\x04node\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_newer_reference() {
    // NEWER_REFERENCE_EXT.
    let input = b"\x83\x5a\x00\x01\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x02";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

// ── Encode-to-buf consistency ──────────────────────────────────────────────

#[test]
fn test_encode_buf_matches_encode_vec() {
    let terms = [
        Term::Int(42),
        Term::Int(-1000),
        Term::Float(std::f64::consts::PI),
        Term::Binary(b"hello world"),
        Term::List(&[Term::Int(1), Term::Int(2), Term::Int(3)]),
        Term::Tuple(&[
            Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"a") }),
            Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"b") }),
        ]),
    ];
    for term in &terms {
        let from_vec = encode_ok(term);
        let from_buf = encode_buf_ok(term);
        assert_eq!(
            from_vec, from_buf,
            "encode_to_buf differs from encode_to_vec"
        );
    }
}
