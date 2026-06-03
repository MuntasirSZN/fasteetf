// ── Shared helpers for integration tests ────────────────────────────────────
//
// Each test binary (tests/*.rs) compiles this module independently, so not
// every helper is used by every binary.  Suppress the resulting noise.

#![allow(dead_code)]

use core::mem::MaybeUninit;
use fasteetf::*;

/// Parse `input` and pass the resulting `Term` to `f`.  The arena used during
/// parsing lives on the caller's stack and is cleaned up when `f` returns.
pub fn with_parse<R>(input: &[u8], f: impl FnOnce(Term<'_>) -> R) -> R {
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let term = parse_etf(ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: Limits::default(),
    })
    .unwrap();
    f(term)
}

/// Parse input that is expected to be malformed, returning the error.
pub fn parse_err(input: &[u8]) -> EtfError {
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    parse_etf(ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: Limits::default(),
    })
    .unwrap_err()
}

/// Encode a term into a `Vec`, panicking on error.
pub fn encode_ok(term: &Term<'_>) -> Vec<u8> {
    encode_to_vec(term).unwrap()
}

/// Encode into a fixed buffer and return the written bytes, panicking on
/// error.  Useful for also exercising the `encode_to_buf` code path.
pub fn encode_buf_ok(term: &Term<'_>) -> Vec<u8> {
    let mut buf = vec![0u8; 65_536];
    let n = encode_to_buf(term, &mut buf).unwrap();
    buf[..n].to_vec()
}
