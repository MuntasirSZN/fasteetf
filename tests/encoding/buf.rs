use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

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

// ── Cover all `encode_term` paths via `encode_to_buf` ──────────────────────
//
// `encode_to_vec` exercises `encode_term_vec`, but `encode_to_buf` exercises a
// separate dispatch (`encode_term`).  Hitting every variant in `encode_to_buf`
// is what the coverage report needs.

#[test]
fn test_encode_buf_small_big() {
    // 2-byte bignum.
    let digits = [0u8, 1];
    let term = Term::BigInt {
        sign: 0,
        digits: &digits,
    };
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 110, 2, 0, 0, 1]);
    // Roundtrip.
    with_parse(&encoded, |t| assert!(matches!(t, Term::BigInt { .. })));
}

#[test]
fn test_encode_buf_large_big() {
    // SmallBigInt: digit count <= 255 uses SMALL_BIG_EXT (110)
    let digits = [0u8, 0, 0, 42];
    let term = Term::BigInt {
        sign: 0,
        digits: &digits,
    };
    let encoded = encode_buf_ok(&term);
    // 131 110 4 0 [0, 0, 0, 42] - SMALL_BIG_EXT with 4 digits
    assert_eq!(encoded, &[131, 110, 4, 0, 0, 0, 0, 42]);
}

#[test]
fn test_encode_buf_improper_list() {
    // New representation: single slice with tail as last element
    let elements = [Term::Int(1), Term::Int(2), Term::Int(3)]; // last element is tail
    let term = Term::ImproperList(&elements);
    let encoded = encode_buf_ok(&term);
    // 131 108 0 0 0 2 97 1 97 2 97 3
    assert_eq!(encoded, &[131, 108, 0, 0, 0, 2, 97, 1, 97, 2, 97, 3]);
}

#[test]
fn test_encode_buf_map() {
    let pairs = [
        (
            Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"a") }),
            Term::Int(1),
        ),
        (
            Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"b") }),
            Term::Int(2),
        ),
    ];
    let term = Term::Map(&pairs);
    let encoded = encode_buf_ok(&term);
    // 131 116 0 0 0 2 119 1 a 97 1 119 1 b 97 2
    let expected = &[
        131, 116, 0, 0, 0, 2, 119, 1, b'a', 97, 1, 119, 1, b'b', 97, 2,
    ];
    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_buf_bit_binary() {
    let data = [0b1010_0000u8];
    let term = Term::BitBinary {
        bits: 3,
        data: &data,
    };
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 77, 0, 0, 0, 1, 3, 0b1010_0000]);
}

#[test]
fn test_encode_buf_pid() {
    // PID_EXT (103) with a 9-byte payload.
    let data = [0u8; 9];
    let mut full_data = vec![103u8];
    full_data.extend_from_slice(&data);
    let term = Term::Pid(&full_data);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 103);
    assert_eq!(&encoded[2..], &data[..]);
}

#[test]
fn test_encode_buf_port() {
    // PORT_EXT (102) with a 5-byte payload.
    let data = [1u8, 2, 3, 4, 5];
    let mut full_data = vec![102u8];
    full_data.extend_from_slice(&data);
    let term = Term::Port(&full_data);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 102, 1, 2, 3, 4, 5]);
}

#[test]
fn test_encode_buf_ref() {
    // NEW_REFERENCE_EXT (114) with a 4-byte payload.
    let data = [0u8, 0, 0, 7];
    let mut full_data = vec![114u8];
    full_data.extend_from_slice(&data);
    let term = Term::Ref(&full_data);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 114, 0, 0, 0, 7]);
}

#[test]
fn test_encode_buf_function_new_fun() {
    // NEW_FUN_EXT (112): tag + Size(4) + payload.
    // Size = 4 (payload length)
    let payload = [1u8, 2, 3, 4];
    let mut full_data = vec![112u8]; // NEW_FUN_EXT tag
    full_data.extend_from_slice(&4u32.to_be_bytes()); // Size = 4
    full_data.extend_from_slice(&payload); // Payload
    let term = Term::Function(&full_data);
    let encoded = encode_buf_ok(&term);
    // Encoder writes data as-is
    assert_eq!(encoded, &[131, 112, 0, 0, 0, 4, 1, 2, 3, 4]);
}

#[test]
fn test_encode_buf_function_export() {
    // EXPORT_EXT (113): tag + raw bytes.
    let payload = [0x77, 3, b'a', b'b', b'c'];
    let mut full_data = vec![113u8];
    full_data.extend_from_slice(&payload);
    let term = Term::Function(&full_data);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 113, 0x77, 3, b'a', b'b', b'c']);
}

#[test]
fn test_encode_buf_record() {
    let payload = [1u8, 2, 3, 4, 5];
    let mut full_data = vec![67u8];
    full_data.extend_from_slice(&payload);
    let term = Term::Record(&full_data);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 67, 1, 2, 3, 4, 5]);
}

#[test]
fn test_encode_buf_buffer_too_small() {
    // Reserve a buffer that is too small to hold even the magic byte + a tag.
    let mut buf = [0u8; 1];
    let term = Term::Int(42);
    let err = encode_to_buf(&term, &mut buf).unwrap_err();
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_encode_buf_empty_payload() {
    // The `write_bytes` helper returns Ok(()) for an empty slice without
    // touching the buffer.
    let term = Term::Binary(&[]);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded, &[131, 109, 0, 0, 0, 0]);
}

#[test]
fn test_encode_buf_atom_large() {
    // 300-byte atom hits the `ATOM_UTF8_EXT` (118) branch of `encode_atom`.
    let bytes: Vec<u8> = (0usize..300).map(|i| (i % 26) as u8 + b'a').collect();
    let a = unsafe { AtomUtf8::from_bytes_unchecked(&bytes) };
    let term = Term::Atom(a);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 118);
    let len = u16::from_be_bytes([encoded[2], encoded[3]]);
    assert_eq!(len, 300);
    assert_eq!(&encoded[4..], &bytes[..]);
}

#[test]
fn test_encode_buf_tuple_large() {
    // 300-element tuple hits the `LARGE_TUPLE_EXT` (105) branch.
    let elements: std::vec::Vec<Term> = (0..300).map(Term::Int).collect();
    let term = Term::Tuple(&elements);
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 105);
    let arity = u32::from_be_bytes([encoded[2], encoded[3], encoded[4], encoded[5]]);
    assert_eq!(arity, 300);
}

#[test]
fn test_encode_buf_small_big_too_large_upgrades() {
    // 256-byte bignum: `encode_small_big` should upgrade to LARGE_BIG_EXT.
    let digits = [0u8; 256];
    let term = Term::BigInt {
        sign: 0,
        digits: &digits,
    };
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 111); // LARGE_BIG_EXT
}

#[test]
fn test_encode_buf_large_big_int() {
    // BigInt with <= 255 digits uses SMALL_BIG_EXT (110)
    let digits = [0u8, 1, 2, 3, 4];
    let term = Term::BigInt {
        sign: 1,
        digits: &digits,
    };
    let encoded = encode_buf_ok(&term);
    assert_eq!(encoded[0], 131);
    assert_eq!(encoded[1], 110); // SMALL_BIG_EXT
    // 1-byte length = 5
    assert_eq!(encoded[2], 5);
    // sign byte
    assert_eq!(encoded[3], 1);
    // digits
    assert_eq!(&encoded[4..], &[0, 1, 2, 3, 4]);
}

#[test]
fn test_encode_large_big_int_roundtrip() {
    // Roundtrip a LargeBigInt through encode_to_vec and parse.
    let digits = [0u8, 0xFF, 0x42];
    let term = Term::BigInt {
        sign: 0,
        digits: &digits,
    };
    let encoded = encode_ok(&term);
    with_parse(&encoded, |parsed| {
        assert!(matches!(
            parsed,
            Term::BigInt { sign: 0, digits: d } if d == [0, 0xFF, 0x42]
        ));
    });
}
