// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for the compression (COMPRESSED tag) code path.
//
// These tests build a zlib-compressed ETF byte sequence by hand and feed it
// into the parser.  The compression backend is intentionally decoupled from
// the one selected at compile time by `fasteetf` — we use `zlib-rs` from
// dev-dependencies to compress the fixtures, and we also exercise the
// [`ZlibBackend`] trait by passing the function pointer through
// `ParseOptions::zlib_backend`.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]
#![cfg(feature = "compression")]

mod common;
use common::*;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};
use fasteetf::{
    EtfError, Limits, ParseOptions, Term, ZlibBackend, encode_to_compressed, parse_etf,
};

/// Counter shared between the `CountingBackend` impl and the test body
/// that asserts on it.  Bumping it on every call proves the parser
/// dispatched to the user-supplied runtime backend.
static RUNTIME_BACKEND_CALLS: AtomicUsize = AtomicUsize::new(0);

/// A trivial [`ZlibBackend`] that defers to `zlib-rs`'s one-shot API.
///
/// Used to verify that the runtime backend override is honoured by the
/// parser regardless of the `zlib-*` feature compiled in.
struct ZlibRsBackend;

impl ZlibBackend for ZlibRsBackend {
    fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
        let (_, rc) = zlib_rs::decompress_slice(target, input, Default::default());
        if rc != zlib_rs::ReturnCode::Ok {
            return Err(EtfError::DecompressionFailed);
        }
        Ok(())
    }
}

/// A backend that records how many times it is called and otherwise
/// defers to the real zlib-rs implementation.  Used to prove the runtime
/// hook is honoured over any compile-time default.
struct CountingBackend;

impl ZlibBackend for CountingBackend {
    fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
        RUNTIME_BACKEND_CALLS.fetch_add(1, Ordering::SeqCst);
        ZlibRsBackend::decompress(target, input)
    }
}

/// Compress `input` with zlib, returning a freshly-allocated `Vec<u8>`.
fn compress_zlib(input: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; zlib_rs::compress_bound(input.len())];
    let (compressed, rc) = zlib_rs::compress_slice(&mut buf, input, Default::default());
    assert_eq!(
        rc,
        zlib_rs::ReturnCode::Ok,
        "compress_slice failed (rc = {rc:?})"
    );
    compressed.to_vec()
}

/// Build a COMPRESSED-tagged ETF byte sequence wrapping `inner`.
///
/// The wire format is:
///
/// ```text
/// 131                       magic
/// 80                        COMPRESSED tag
/// <4 bytes BE>              UncompressedSize
/// <zlib stream>             zlib-wrapped `inner` payload
/// ```
fn compressed_etf(inner: &[u8]) -> Vec<u8> {
    let compressed = compress_zlib(inner);
    let mut out = Vec::with_capacity(1 + 1 + 4 + compressed.len());
    out.push(131);
    out.push(0x50); // COMPRESSED tag
    out.extend_from_slice(&(inner.len() as u32).to_be_bytes());
    out.extend_from_slice(&compressed);
    out
}

fn make_arena() -> Vec<MaybeUninit<u8>> {
    vec![MaybeUninit::<u8>::uninit(); 65536]
}

// ── Compile-time backend ──────────────────────────────────────────────────

#[cfg(any(
    feature = "zlib-rs",
    feature = "miniz_oxide",
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
#[test]
fn decompresses_via_compile_time_backend() {
    // Inner: the small integer 42 (the ETF tag + payload, with no magic
    // byte — the COMPRESSED wrapper carries a *term*, not a full stream).
    let inner = [97, 42];
    let wire = compressed_etf(&inner);

    let mut decomp = vec![0u8; inner.len()];
    let mut arena = make_arena();

    let term = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        // `None` ⇒ use the compile-time selected backend.
        zlib_backend: None,
    })
    .expect("compressed input should parse via the compile-time backend");

    assert!(matches!(term, Term::Int(42)));
}

#[cfg(any(
    feature = "zlib-rs",
    feature = "miniz_oxide",
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
#[test]
fn decompresses_via_streaming_with_compile_time_backend() {
    use fasteetf::parse_etf_streaming;

    let inner = [97, 42];
    let wire = compressed_etf(&inner);

    let mut decomp = vec![0u8; inner.len()];
    let mut arena = make_arena();

    let term = parse_etf_streaming(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .expect("compressed input should parse via the compile-time streaming backend");

    assert!(matches!(term, Term::Int(42)));
}

#[cfg(not(any(
    feature = "zlib-rs",
    feature = "miniz_oxide",
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
)))]
#[test]
fn no_backend_yields_unsupported_tag() {
    // No `zlib-*` feature is enabled and the caller did not supply a
    // runtime backend.  The COMPRESSED wrapper must surface as
    // `UnsupportedTag(80)` (the COMPRESSED tag) rather than silently
    // succeeding or panicking.
    let inner = [97, 42];
    let wire = compressed_etf(&inner);

    let mut decomp = vec![0u8; inner.len()];
    let mut arena = make_arena();

    let err = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::UnsupportedTag(80)));
}

// ── Runtime backend override ──────────────────────────────────────────────

#[test]
fn decompresses_via_runtime_backend() {
    let inner = [97, 42];
    let wire = compressed_etf(&inner);

    let mut decomp = vec![0u8; inner.len()];
    let mut arena = make_arena();

    let term = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        // Force the use of the trait-based backend even when the crate
        // has a built-in one compiled in.
        zlib_backend: Some(<ZlibRsBackend as ZlibBackend>::decompress),
    })
    .expect("compressed input should parse via the runtime backend");

    assert!(matches!(term, Term::Int(42)));
}

// ── Roundtrip with bigger payloads ────────────────────────────────────────

#[test]
fn roundtrip_large_compressed_term() {
    // Encode `[1, 2, ..., 100]` then wrap it in COMPRESSED.
    let mut inner = vec![108, 0, 0, 0, 100];
    for i in 1..=100u8 {
        inner.extend_from_slice(&[97, i]);
    }
    inner.extend_from_slice(&[106]); // NIL tail
    let wire = compressed_etf(&inner);

    let mut decomp = vec![0u8; inner.len()];
    let mut arena = make_arena();

    let term = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: Some(<ZlibRsBackend as ZlibBackend>::decompress),
    })
    .expect("large compressed term should parse");

    match term {
        Term::List(items) => {
            assert_eq!(items.len(), 100, "expected 100 list elements");
        }
        other => panic!("expected List, got {other:?}"),
    }
}

// ── Error paths ───────────────────────────────────────────────────────────

#[test]
fn missing_decompression_buffer_is_an_error() {
    let wire = compressed_etf(&[97, 1]);
    let mut arena = make_arena();

    let err = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::InsufficientDecompressionBuffer));
}

#[test]
fn undersized_decompression_buffer_is_an_error() {
    let wire = compressed_etf(&[97, 1]);
    let mut decomp = [0u8; 1]; // too small: the inner term is 2 bytes
    let mut arena = make_arena();

    let err = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::InsufficientDecompressionBuffer));
}

#[test]
fn corrupted_zlib_payload_is_a_decompression_error() {
    // Build a wire frame with a payload that is *not* valid zlib.
    let mut wire = vec![131, 0x50, 0, 0, 0, 5];
    wire.extend_from_slice(&[0xff; 5]);

    let mut decomp = vec![0u8; 5];
    let mut arena = make_arena();

    let err = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: Some(<ZlibRsBackend as ZlibBackend>::decompress),
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::DecompressionFailed));
}

#[test]
fn size_mismatch_with_payload_is_a_decompression_error() {
    // Inner payload is 4 bytes, but the wire header claims 3.  The
    // parser will hand a 3-byte buffer to the decompressor, which will
    // not be able to drain the full zlib stream.
    let inner = [97, 42, 0x21, 0x21];
    let compressed = compress_zlib(&inner);

    let mut wire = vec![131, 0x50, 0, 0, 0, 3];
    wire.extend_from_slice(&compressed);
    let mut decomp = vec![0u8; 3];
    let mut arena = make_arena();

    let err = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: Some(<ZlibRsBackend as ZlibBackend>::decompress),
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::DecompressionFailed));
}

// ── Trait dispatch is actually invoked ────────────────────────────────────

#[test]
fn runtime_backend_is_actually_called() {
    RUNTIME_BACKEND_CALLS.store(0, Ordering::SeqCst);

    let inner = [97, 42];
    let wire = compressed_etf(&inner);

    let mut decomp = vec![0u8; inner.len()];
    let mut arena = make_arena();

    let term = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: Some(<CountingBackend as ZlibBackend>::decompress),
    })
    .unwrap();

    assert!(matches!(term, Term::Int(42)));
    assert!(
        RUNTIME_BACKEND_CALLS.load(Ordering::SeqCst) >= 1,
        "runtime ZlibBackend was not invoked"
    );
}

// ── encode_to_compressed roundtrips ───────────────────────────────────

/// zlib-rs' one-shot compress, exposed as a `ZlibCompressFn` so the
/// runtime-backend path of `encode_to_compressed` is exercised even
/// when the crate is built with a different compile-time backend.
fn zlib_rs_compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    let target_len = target.len();
    let (tail, rc) = zlib_rs::compress_slice(target, input, Default::default());
    if rc != zlib_rs::ReturnCode::Ok {
        return Err(EtfError::CompressionFailed);
    }
    Ok(target_len - tail.len())
}

/// A `ZlibDecompressFn` that delegates to zlib-rs, used to verify that
/// the wire produced by `encode_to_compressed` roundtrips through a
/// runtime-supplied backend.
fn zlib_rs_decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    let (_, rc) = zlib_rs::decompress_slice(target, input, Default::default());
    if rc != zlib_rs::ReturnCode::Ok {
        return Err(EtfError::DecompressionFailed);
    }
    Ok(())
}

#[test]
fn encode_to_compressed_roundtrip_with_compile_time_backend() {
    let term = Term::Int(42);

    // `encode_to_compressed` writes the term's ETF bytes (including
    // the leading magic) into `intermediate`.  Use a plain zeroed
    // buffer since the encoder fills it from offset 0.
    let mut intermediate = [0u8; 1024];
    // `compressBound` over-estimates — 1024 is plenty for an integer.
    let mut output = [0u8; 1024];

    let n = encode_to_compressed(&term, &mut intermediate, &mut output, None)
        .expect("encode_to_compressed should succeed with the compile-time backend");
    let wire = &output[..n];

    // The wire must start with the ETF magic and the COMPRESSED tag.
    assert_eq!(wire[0], 131);
    assert_eq!(wire[1], 80); // COMPRESSED

    // The uncompressed-size field must equal the bare-encoded term's
    // length (without the leading magic byte).  For `Term::Int(42)` the
    // bare encoding is `97 42` — two bytes.
    let uncomp_size = u32::from_be_bytes([wire[2], wire[3], wire[4], wire[5]]);
    assert_eq!(uncomp_size, 2);

    // Roundtrip: parse the COMPRESSED wire back to a Term.
    let mut decomp = [0u8; 64];
    let mut arena = make_arena();
    let parsed = parse_etf(ParseOptions {
        input: wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .expect("compressed wire should parse");
    assert!(matches!(parsed, Term::Int(42)));
}

#[test]
fn encode_to_compressed_roundtrip_with_runtime_backend() {
    let term = Term::Int(2026);
    let mut intermediate = [0u8; 1024];
    let mut output = [0u8; 1024];

    let n = encode_to_compressed(
        &term,
        &mut intermediate,
        &mut output,
        Some(zlib_rs_compress),
    )
    .expect("encode_to_compressed should succeed with the runtime backend");
    let wire = &output[..n];

    let mut decomp = [0u8; 64];
    let mut arena = make_arena();
    let parsed = parse_etf(ParseOptions {
        input: wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: Some(zlib_rs_decompress),
    })
    .expect("compressed wire should parse with the runtime backend");
    assert!(matches!(parsed, Term::Int(2026)));
}

#[test]
fn encode_to_compressed_uncompressed_size_matches_encoded_term() {
    // Build a more interesting term so the uncompressed size is not
    // trivially small.
    let term = Term::List(&[
        Term::Int(1),
        Term::Int(2),
        Term::Int(3),
        Term::Int(4),
        Term::Int(5),
    ]);
    let mut intermediate = [0u8; 1024];
    let mut output = [0u8; 1024];

    let n = encode_to_compressed(&term, &mut intermediate, &mut output, None).unwrap();
    let wire = &output[..n];
    let uncomp_size = u32::from_be_bytes([wire[2], wire[3], wire[4], wire[5]]) as usize;

    // Compare against a bare `encode_to_buf` output to verify the size
    // field matches exactly.  The bare encoding has a leading magic
    // byte (131) that the COMPRESSED wrapper does not include in the
    // uncompressed size.
    let bare = encode_buf_ok(&term);
    assert_eq!(uncomp_size, bare.len() - 1);
}
