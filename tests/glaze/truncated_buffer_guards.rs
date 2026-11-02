// Truncated buffer guards — only error-path tests (no lifetime issues)

use fasteetf::{EtfError, Limits, ParseOptions, parse_etf};

fn parse_err(input: &[u8]) -> EtfError {
    let arena = bumpalo::Bump::<16>::with_min_align_and_capacity(65536);
    arena.set_allocation_limit(Some(65536));
    #[cfg(feature = "compression")]
    let options = ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &arena,
        limits: Limits::default(),
        zlib_backend: None,
    };
    #[cfg(not(feature = "compression"))]
    let options = ParseOptions {
        input,
        ast_arena: &arena,
        limits: Limits::default(),
    };
    parse_etf(options).unwrap_err()
}

#[test]
fn test_truncated_small_int() {
    let err = parse_err(&[0x83, 0x61]);
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_truncated_map() {
    let err = parse_err(&[0x83, 0x74, 0x00, 0x00, 0x00, 0x01]);
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_truncated_small_big_integer() {
    for input in [
        &[0x83, 0x6E][..],
        &[0x83, 0x6E, 0x01][..],
        &[0x83, 0x6E, 0x08, 0x00][..],
    ] {
        assert!(matches!(parse_err(input), EtfError::UnexpectedEof));
    }
}

#[test]
fn test_empty_buffer() {
    assert!(matches!(parse_err(&[]), EtfError::UnexpectedEof));
}
