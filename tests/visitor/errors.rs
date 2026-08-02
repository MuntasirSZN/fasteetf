use super::*;

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
