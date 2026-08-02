use crate::error::EtfError;

#[inline]
pub(crate) fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    // `decompress_slice` runs the inflate state machine over `input`
    // and writes up to `target.len()` bytes.  The `rust-allocator`
    // feature on zlib-rs is propagated from our `alloc` feature (see
    // `Cargo.toml`); when `alloc` is off it is not enabled, and the
    // call below stays heap-free.
    let (_, rc) = ::zlib_rs::decompress_slice(target, input, Default::default());
    if rc != ::zlib_rs::ReturnCode::Ok {
        return Err(EtfError::DecompressionFailed);
    }
    Ok(())
}

/// One-shot zlib compression via `zlib-rs`'s streaming `deflate`.
///
/// Available only when the `alloc` feature is on, because
/// `compress_slice` constructs a `z_stream` that holds a heap-backed
/// internal state (the window, hash tables, etc.).  Our `alloc`
/// feature propagates `rust-allocator` from zlib-rs, which is the
/// allocator the stream uses.
#[cfg(feature = "alloc")]
#[inline]
pub(crate) fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    // `compress_slice` writes a zlib-wrapped (header + adler32) deflate
    // stream into `target` and returns the subslice of `target` holding
    // the compressed bytes; its length is the compressed size.
    let (compressed, rc) = ::zlib_rs::compress_slice(target, input, Default::default());
    if rc != ::zlib_rs::ReturnCode::Ok {
        return Err(EtfError::CompressionFailed);
    }
    Ok(compressed.len())
}

#[cfg(test)]
#[cfg(not(miri))]
mod tests {
    use super::*;

    #[test]
    fn decompress_corrupt_input() {
        let mut out = [0u8; 16];
        let err = decompress(&mut out, b"not a zlib stream").unwrap_err();
        assert!(matches!(err, EtfError::DecompressionFailed));
    }

    // Compression needs the `alloc` feature (it drives the zlib-rs stream).
    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;
        use alloc::vec;

        #[test]
        fn roundtrip() {
            let input = b"hello zlib-rs roundtrip payload";
            let mut compressed = vec![0u8; ::zlib_rs::compress_bound(input.len())];
            let n = compress(&mut compressed, input).unwrap();
            assert!(n > 0 && n < compressed.len());
            let mut out = vec![0u8; input.len()];
            decompress(&mut out, &compressed[..n]).unwrap();
            assert_eq!(&out, input);
        }

        #[test]
        fn compress_undersized_target() {
            // 256 incompressible bytes cannot fit in a 4-byte target.
            let input = [0xABu8; 256];
            let mut target = [0u8; 4];
            let err = compress(&mut target, &input).unwrap_err();
            assert!(matches!(err, EtfError::CompressionFailed));
        }
    }
}
