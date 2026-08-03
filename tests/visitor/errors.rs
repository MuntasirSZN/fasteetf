use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

// ── Error paths ─────────────────────────────────────────────────────────────

#[test]
fn test_visitor_invalid_magic() {
    let mut v = EventLog::default();
    let err = run_visitor(b"\x00\x61\x01", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::InvalidMagicNumber));
}

#[test]
fn test_visitor_truncated() {
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\x61", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_visitor_unknown_tag() {
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\xff", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnsupportedTag(0xff)));
}

#[test]
fn test_visitor_depth_limit() {
    // 129 nested tuples — exceeds the default max_depth of 128.
    let mut buf = vec![131u8];
    for _ in 0..129 {
        buf.push(104);
        buf.push(1);
    }
    buf.push(97);
    buf.push(0);
    let mut v = EventLog::default();
    let err = run_visitor(&buf, &mut v).unwrap_err();
    assert!(matches!(err, EtfError::RecursionLimitExceeded));
}

#[test]
fn test_visitor_atom_too_large() {
    // Use a tight `max_atom_len` so we can construct a valid small buffer
    // whose length exceeds it.  SMALL_ATOM_UTF8_EXT (119) is enough.
    let buf = vec![131, 119, 3, b'a', b'b', b'c']; // 3-byte atom
    let tight = Limits {
        max_atom_len: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::AtomTooLarge));
}

#[test]
fn test_visitor_string_too_large() {
    // STRING_EXT (107) with a length > max_string_len.  Use a tight limit so
    // we can construct a small buffer that trips the check.
    let buf = vec![131, 107, 0, 4, b'a', b'b', b'c', b'd']; // 4-byte string
    let tight = Limits {
        max_string_len: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_binary_too_large() {
    // BINARY_EXT (109) with a length > max_binary_size.  Use a tight limit
    // so we can trip the check with a small buffer.
    let buf = vec![131, 109, 0, 0, 0, 4, 1, 2, 3, 4]; // 4-byte binary
    let tight = Limits {
        max_binary_size: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_list_too_large() {
    // LIST_EXT (108) with len > max_list_len.  Use a tight limit.
    let buf = vec![131, 108, 0, 0, 0, 3, 97, 1, 97, 2, 97, 3, 106]; // 3-elem list
    let tight = Limits {
        max_list_len: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_map_too_large() {
    // MAP_EXT (116) with arity > max_map_len.  Use a tight limit.
    let buf = vec![131, 116, 0, 0, 0, 2, 97, 1, 97, 2, 97, 3, 97, 4]; // 2 pairs
    let tight = Limits {
        max_map_len: 1,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::MapTooLarge));
}

#[test]
fn test_visitor_tuple_too_large() {
    // LARGE_TUPLE_EXT (105) with arity > max_tuple_arity.  Use a tight limit.
    let buf = vec![131, 105, 0, 0, 0, 3, 97, 1, 97, 2, 97, 3]; // 3-tuple
    let tight = Limits {
        max_tuple_arity: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::TupleTooLarge));
}

#[test]
fn test_visitor_small_big_too_large() {
    // LARGE_BIG_EXT (111) with len > max_binary_size.  Use a tight limit.
    let buf = vec![131, 111, 0, 0, 0, 4, 0, 1, 2, 3]; // 4-digit bignum
    let tight = Limits {
        max_binary_size: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_ref_too_large() {
    // NEW_REFERENCE_EXT (114) with len > max_reference_words.  Use a tight
    // limit so the test runs in microseconds.
    let buf = vec![131, 114, 0, 3]; // 3 words
    let tight = Limits {
        max_reference_words: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_new_fun_too_large() {
    // NEW_FUN_EXT: Size (after subtracting 4 for the Size field itself)
    // exceeds max_fun_size.  Use a tight limit.
    let buf = vec![131, 112, 0, 0, 0, 6]; // remaining = 2, limit 1
    let tight = Limits {
        max_fun_size: 1,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_invalid_fun_size() {
    // NEW_FUN_EXT: Size < 4 -> InvalidSize.
    let buf = vec![131, 112, 0, 0, 0, 3];
    let mut v = EventLog::default();
    let err = run_visitor(&buf, &mut v).unwrap_err();
    assert!(matches!(err, EtfError::InvalidSize));
}

#[test]
fn test_visitor_invalid_legacy_float() {
    // FLOAT_EXT (99): 31 bytes that don't form a parseable float.
    let mut buf = vec![131, 99];
    buf.extend(std::iter::repeat_n(b'x', 31));
    let mut v = EventLog::default();
    let err = run_visitor(&buf, &mut v).unwrap_err();
    assert!(matches!(err, EtfError::InvalidFloat));
}

#[test]
fn test_visitor_local_ext_unsupported() {
    // LOCAL_EXT (121) is reported as UnsupportedTag.
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\x79", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnsupportedTag(121)));
}

#[test]
fn test_visitor_atom_cache_ref_unsupported() {
    // ATOM_CACHE_REF (82) is reported as UnsupportedTag.
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\x52\x00", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnsupportedTag(82)));
}

// ── Remaining limit variants ────────────────────────────────────────────────

#[test]
fn test_visitor_atom_utf8_ext_too_large() {
    // ATOM_UTF8_EXT (118) with len > max_atom_len (SMALL_ATOM_UTF8_EXT is
    // covered above).
    let buf = vec![131, 118, 0, 3, b'a', b'b', b'c'];
    let tight = Limits {
        max_atom_len: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::AtomTooLarge));
}

#[test]
fn test_visitor_small_big_ext_too_large() {
    // SMALL_BIG_EXT (110) with len > max_binary_size.
    let buf = vec![131, 110, 4, 0, 1, 2, 3, 4]; // 4-digit bignum
    let tight = Limits {
        max_binary_size: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_bit_binary_too_large() {
    // BIT_BINARY_EXT (77) with a data length > max_bit_binary_size.
    let buf = vec![131, 77, 0, 0, 0, 4, 4, 0xAB, 0xCD, 0xEF, 0x12];
    let tight = Limits {
        max_bit_binary_size: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_small_tuple_too_large() {
    // SMALL_TUPLE_EXT (104) with arity > max_tuple_arity.
    let buf = vec![131, 104, 3, 97, 1, 97, 2, 97, 3];
    let tight = Limits {
        max_tuple_arity: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::TupleTooLarge));
}

#[test]
fn test_visitor_newer_reference_too_large() {
    // NEWER_REFERENCE_EXT (90) with len > max_reference_words.  Mirrors the
    // NEW_REFERENCE_EXT case, including the shared ListTooLarge mapping.
    let buf = vec![131, 90, 0, 3]; // 3 words
    let tight = Limits {
        max_reference_words: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_record_too_large() {
    // RECORD_EXT (67) with more fields than max_map_len.
    let buf = vec![131, 67, 0, 0, 0, 3, 0, 97, 1, 97, 2, 97, 3]; // 3 fields
    let tight = Limits {
        max_map_len: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::MapTooLarge));
}

// ── Compressed input ────────────────────────────────────────────────────────

/// Build a COMPRESSED-tagged ETF byte sequence wrapping `inner` (magic,
/// tag, BE u32 uncompressed size, zlib stream).  Uses the `zlib-rs`
/// dev-dependency, mirroring `tests/compression/mod.rs`.
#[cfg(feature = "compression")]
fn compressed_visitor_wire(inner: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; zlib_rs::compress_bound(inner.len())];
    let (compressed, rc) = zlib_rs::compress_slice(&mut buf, inner, Default::default());
    assert_eq!(rc, zlib_rs::ReturnCode::Ok);
    let mut out = Vec::with_capacity(6 + compressed.len());
    out.push(131);
    out.push(0x50); // COMPRESSED
    out.extend_from_slice(&(inner.len() as u32).to_be_bytes());
    out.extend_from_slice(compressed);
    out
}

/// A runtime [`ZlibBackend`]-style decompressor backed by the `zlib-rs`
/// dev-dependency.  Used so the tests pass regardless of which (if any)
/// `zlib-*` feature is compiled into the crate.
#[cfg(feature = "compression")]
fn zlib_rs_decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    let (_, rc) = zlib_rs::decompress_slice(target, input, Default::default());
    if rc != zlib_rs::ReturnCode::Ok {
        return Err(EtfError::DecompressionFailed);
    }
    Ok(())
}

#[cfg(feature = "compression")]
#[test]
fn test_visitor_compressed_missing_buffer() {
    let wire = compressed_visitor_wire(&[97, 42]);
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(
        &wire,
        None,
        Some(zlib_rs_decompress),
        &mut v,
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(err, EtfError::InsufficientDecompressionBuffer));
}

#[cfg(feature = "compression")]
#[test]
fn test_visitor_compressed_undersized_buffer() {
    let wire = compressed_visitor_wire(&[97, 42]);
    let mut decomp = [0u8; 1]; // inner term needs 2 bytes
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(
        &wire,
        Some(&mut decomp),
        Some(zlib_rs_decompress),
        &mut v,
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(err, EtfError::InsufficientDecompressionBuffer));
}

#[cfg(feature = "compression")]
#[test]
fn test_visitor_compressed_roundtrip() {
    // Inner: the list [1, 2].
    let inner = [108, 0, 0, 0, 2, 97, 1, 97, 2, 106];
    let wire = compressed_visitor_wire(&inner);
    let mut decomp = [0u8; 64];
    let mut v = EventLog::default();
    parse_etf_with_visitor(
        &wire,
        Some(&mut decomp),
        Some(zlib_rs_decompress),
        &mut v,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(
        v.events,
        vec!["list_start(len=2)", "int(1)", "int(2)", "list_end"]
    );
}
