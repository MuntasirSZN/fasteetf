// Truncated buffer guards — only error-path tests (no lifetime issues)

use core::mem::MaybeUninit;
use fasteetf::{EtfError, parse_etf};

fn parse_err(input: &[u8]) -> EtfError {
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let options = fasteetf::ParseOptions {
        input,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),
        decompressed_buffer: None,
        zlib_backend: None,
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
