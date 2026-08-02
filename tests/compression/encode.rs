use super::*;

// ── encode_to_compressed roundtrips ───────────────────────────────────

/// zlib-rs' one-shot compress, exposed as a `ZlibCompressFn` so the
/// runtime-backend path of `encode_to_compressed` is exercised even
/// when the crate is built with a different compile-time backend.
///
/// `compress_slice` returns the subslice holding the compressed bytes,
/// so its length *is* the compressed size.
fn zlib_rs_compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    let (compressed, rc) = zlib_rs::compress_slice(target, input, Default::default());
    if rc != zlib_rs::ReturnCode::Ok {
        return Err(EtfError::CompressionFailed);
    }
    Ok(compressed.len())
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

#[cfg(not(miri))]
#[cfg(any(
    all(feature = "zlib-rs", feature = "alloc"),
    all(feature = "miniz_oxide", feature = "alloc"),
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
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

#[cfg(not(miri))]
#[cfg(any(
    all(feature = "zlib-rs", feature = "alloc"),
    all(feature = "miniz_oxide", feature = "alloc"),
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
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

#[test]
fn encode_to_compressed_undersized_output() {
    // The COMPRESSED header alone needs 6 bytes (magic, tag, size).
    let term = Term::Int(42);
    let mut intermediate = [0u8; 64];
    let mut output = [0u8; 5];
    let err = encode_to_compressed(&term, &mut intermediate, &mut output, None).unwrap_err();
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[cfg(not(miri))]
#[cfg(any(
    all(feature = "zlib-rs", feature = "alloc"),
    all(feature = "miniz_oxide", feature = "alloc"),
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
#[test]
fn encode_to_compressed_compress_failure() {
    // An incompressible 256-byte payload cannot fit into the single
    // byte left after the 6-byte COMPRESSED header: the backend must
    // report CompressionFailed.
    let bytes: Vec<u8> = (0u8..=255).collect();
    let term = Term::Binary(&bytes);
    let mut intermediate = [0u8; 512];
    let mut output = [0u8; 7];
    let err = encode_to_compressed(&term, &mut intermediate, &mut output, None).unwrap_err();
    assert!(matches!(err, EtfError::CompressionFailed));
}
