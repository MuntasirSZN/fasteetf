use super::*;

// ── Compile-time backend ──────────────────────────────────────────────────

#[cfg(not(miri))]
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

#[cfg(not(miri))]
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
fn undersized_decompression_buffer_is_an_error_streaming() {
    use fasteetf::parse_etf_streaming;

    let wire = compressed_etf(&[97, 1]);
    let mut decomp = [0u8; 1]; // too small: the inner term is 2 bytes
    let mut arena = make_arena();

    let err = parse_etf_streaming(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::InsufficientDecompressionBuffer));
}

#[cfg(not(miri))]
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
fn corrupted_payload_compile_time_backend() {
    // Same wire as `corrupted_zlib_payload_is_a_decompression_error` but
    // with `zlib_backend: None`, so the failure surfaces from the
    // compile-time backend rather than a runtime override.
    let mut wire = vec![131, 0x50, 0, 0, 0, 5];
    wire.extend_from_slice(&[0xff; 5]);

    let mut decomp = vec![0u8; 5];
    let mut arena = make_arena();

    let err = parse_etf(ParseOptions {
        input: &wire,
        decompressed_buffer: Some(&mut decomp),
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap_err();

    assert!(matches!(err, EtfError::DecompressionFailed));
}

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
