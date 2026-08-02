use super::*;

// ── Error cases ─────────────────────────────────────────────────────────────

#[test]
fn test_invalid_magic() {
    let err = parse_err(b"\x00\x61\x01");
    assert!(matches!(err, EtfError::InvalidMagicNumber));
}

#[test]
fn test_truncated() {
    let err = parse_err(b"\x83\x61");
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_unknown_tag() {
    let err = parse_err(b"\x83\xff");
    assert!(matches!(err, EtfError::UnsupportedTag(255)));
}

#[test]
fn test_depth_limit() {
    let mut buf = vec![131u8];
    for _ in 0..129 {
        buf.push(104);
        buf.push(1);
    }
    buf.push(97);
    buf.push(0);
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::RecursionLimitExceeded));
}

#[test]
fn test_binary_too_large() {
    let buf = vec![131, 109, 4, 16, 0, 0];
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_arena_exhaustion() {
    use core::mem::MaybeUninit;
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 16];
    let input = b"\x83\x68\x0a\x61\x01\x61\x02\x61\x03\x61\x04\x61\x05\
                  \x61\x06\x61\x07\x61\x08\x61\x09\x61\x0a";
    #[cfg(feature = "compression")]
    let options = ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    };
    #[cfg(not(feature = "compression"))]
    let options = ParseOptions {
        input,
        ast_arena: &mut arena,
        limits: Limits::default(),
    };
    let err = parse_etf(options).unwrap_err();
    assert!(matches!(err, EtfError::ArenaExhausted));
}

#[test]
fn test_invalid_fun_size() {
    let buf = vec![131, 112, 0, 0, 0, 3];
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::InvalidSize));
}
