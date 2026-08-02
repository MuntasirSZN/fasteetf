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
    // stream into `target` and returns the unused tail of `target`.
    // The number of compressed bytes is therefore the original
    // `target` length minus the returned tail's length.
    let target_len = target.len();
    let (tail, rc) = ::zlib_rs::compress_slice(target, input, Default::default());
    if rc != ::zlib_rs::ReturnCode::Ok {
        return Err(EtfError::CompressionFailed);
    }
    Ok(target_len - tail.len())
}
