use super::*;

// ── Streaming visitor ───────────────────────────────────────────────────────

#[test]
fn test_visitor_streaming_complete() {
    let mut v = EventLog::default();
    parse_etf_with_visitor_streaming(b"\x83\x61\x2a", None, None, &mut v, &Limits::default())
        .unwrap();
    assert_eq!(v.events, vec!["int(42)"]);
}

#[test]
fn test_visitor_streaming_incomplete() {
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor_streaming(b"\x83", None, None, &mut v, &Limits::default())
        .unwrap_err();
    assert!(matches!(err, EtfError::Incomplete(_)));
}

#[test]
fn test_visitor_streaming_invalid_magic() {
    let mut v = EventLog::default();
    let err =
        parse_etf_with_visitor_streaming(b"\x00\x61\x01", None, None, &mut v, &Limits::default())
            .unwrap_err();
    assert!(matches!(err, EtfError::InvalidMagicNumber));
}

// ── Streaming compressed input ──────────────────────────────────────────────

#[cfg(feature = "compression")]
fn compressed_wire(inner: &[u8]) -> Vec<u8> {
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
/// dev-dependency, so these tests pass with any (or no) `zlib-*` feature.
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
fn test_visitor_streaming_compressed_missing_buffer() {
    let wire = compressed_wire(&[97, 42]);
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor_streaming(
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
fn test_visitor_streaming_compressed_undersized_buffer() {
    let wire = compressed_wire(&[97, 42]);
    let mut decomp = [0u8; 1];
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor_streaming(
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
fn test_visitor_streaming_compressed_roundtrip() {
    let inner = [104, 2, 97, 1, 97, 2]; // {1, 2}
    let wire = compressed_wire(&inner);
    let mut decomp = [0u8; 64];
    let mut v = EventLog::default();
    parse_etf_with_visitor_streaming(
        &wire,
        Some(&mut decomp),
        Some(zlib_rs_decompress),
        &mut v,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(
        v.events,
        vec!["tuple_start(arity=2)", "int(1)", "int(2)", "tuple_end"]
    );
}
