use crate::error::EtfError;

#[inline]
pub(crate) fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    use miniz_oxide::inflate::stream::InflateState;
    use miniz_oxide::{DataFormat, MZFlush, MZStatus};

    let mut state = InflateState::new(DataFormat::Zlib);

    // Total bytes written so far into `target`.  The streaming API
    // hands us a fresh `&mut [u8]` view into the unused tail of
    // `target` on every call, so we keep an absolute index and a
    // shorter slice.
    let mut written: usize = 0;
    let mut in_off: usize = 0;

    loop {
        let out_slice = &mut target[written..];
        let in_slice = &input[in_off..];

        let res =
            ::miniz_oxide::inflate::stream::inflate(&mut state, in_slice, out_slice, MZFlush::None);

        written += res.bytes_written;
        in_off += res.bytes_consumed;

        match res.status {
            Ok(MZStatus::StreamEnd) => {
                // The compressed stream declared its uncompressed
                // size up front in the ETF wrapper, and `target`
                // was sized to match.  A short or long output here
                // indicates corruption.
                if written == target.len() {
                    return Ok(());
                }
                return Err(EtfError::DecompressionFailed);
            }
            Ok(MZStatus::Ok) => {
                // The decompressor made forward progress but the
                // stream is not yet complete.  In a one-shot call
                // with all input supplied, reaching this state
                // without the output filling up means the stream
                // is truncated or has extra trailing data.
                if in_off == input.len() {
                    return Err(EtfError::DecompressionFailed);
                }
                // Otherwise loop and keep going.
            }
            // Any error is treated as a decompression failure.
            Err(_) => return Err(EtfError::DecompressionFailed),
            Ok(_) => return Err(EtfError::DecompressionFailed),
        }
    }
}

/// Streaming compression using `miniz_oxide`'s `CompressorOxide`.
///
/// Available only when the `alloc` feature is on, because
/// `CompressorOxide` embeds a `Box<HuffmanOxide>` for its internal
/// Huffman tables and therefore needs the global allocator to be
/// available at construction time.  Our `alloc` feature propagates
/// `with-alloc` from miniz_oxide.
#[cfg(feature = "alloc")]
#[inline]
pub(crate) fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    use miniz_oxide::deflate::core::CompressorOxide;
    use miniz_oxide::deflate::stream::deflate;
    use miniz_oxide::{MZFlush, MZStatus};

    // `CompressorOxide::default()` configures the zlib wrapper
    // (writes a 2-byte zlib header and a 4-byte adler32 trailer).
    // That is exactly what ETF's COMPRESSED tag expects, so we can
    // write straight into `target` without manual framing.
    let mut compressor = CompressorOxide::default();
    let res = deflate(&mut compressor, input, target, MZFlush::Finish);
    match res.status {
        Ok(MZStatus::StreamEnd) => Ok(res.bytes_written),
        _ => Err(EtfError::CompressionFailed),
    }
}
