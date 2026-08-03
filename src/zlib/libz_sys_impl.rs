use crate::error::EtfError;
use core::ffi::{c_int, c_ulong};

#[inline]
pub(crate) fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    // `target.len()` is the *expected* uncompressed size, declared
    // by the ETF stream header.  On success, `uncompress` updates
    // `out_len` to the actual bytes written.  The function returns
    // `Z_OK` (0) on success and one of the `Z_*_ERROR` otherwise.
    //
    // `out_len` is a 64-bit slot even though the binding types it as
    // `c_ulong` (32-bit on Windows): stock zlib's `uLongf` is 32-bit
    // there while a cloudflare-zlib build linked into the same binary
    // uses a 64-bit `z_size_t`.  A 32-bit implementation reads and
    // writes only the low half of the slot, so both ABIs see the
    // correct capacity and cannot overrun the caller's buffer.
    let mut out_len: usize = target.len();
    let rc: c_int = unsafe {
        ::libz_sys::uncompress(
            target.as_mut_ptr(),
            &mut out_len as *mut usize as *mut c_ulong,
            input.as_ptr(),
            input.len() as c_ulong,
        )
    };
    if rc != 0 {
        return Err(EtfError::DecompressionFailed);
    }
    Ok(())
}

/// Default compression level for `compress2` (6, equivalent to zlib's
/// `Z_DEFAULT_COMPRESSION`).  `libz-sys` does not re-export the
/// `Z_DEFAULT_COMPRESSION` constant without its default features, so
/// we use the literal value.
const Z_DEFAULT_COMPRESSION: c_int = 6;

#[inline]
pub(crate) fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    // `compress2` writes a zlib-wrapped deflate stream into `target`
    // and updates `out_len` to the actual bytes written.  The
    // function returns `Z_OK` (0) on success and one of the
    // `Z_*_ERROR` constants otherwise.  We use the default
    // compression level; a finer-grained level knob can be added
    // later if needed.
    //
    // `out_len` uses the same 64-bit slot as `decompress` above so
    // that both stock zlib's 32-bit `uLongf` and cloudflare-zlib's
    // 64-bit `z_size_t` agree on the buffer capacity.
    let mut out_len: usize = target.len();
    let rc: c_int = unsafe {
        ::libz_sys::compress2(
            target.as_mut_ptr(),
            &mut out_len as *mut usize as *mut c_ulong,
            input.as_ptr(),
            input.len() as c_ulong,
            Z_DEFAULT_COMPRESSION,
        )
    };
    if rc != 0 {
        return Err(EtfError::CompressionFailed);
    }
    Ok(out_len)
}

#[cfg(test)]
#[cfg(not(miri))]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let input = b"hello libz roundtrip payload";
        let mut compressed = [0u8; 128];
        let n = compress(&mut compressed, input).unwrap();
        assert!(n > 0 && n < compressed.len());
        let mut out = [0u8; 64];
        decompress(&mut out, &compressed[..n]).unwrap();
        assert_eq!(&out[..input.len()], input);
    }

    #[test]
    fn decompress_corrupt_input() {
        let mut out = [0u8; 16];
        let err = decompress(&mut out, b"not a zlib stream").unwrap_err();
        assert!(matches!(err, EtfError::DecompressionFailed));
    }

    #[test]
    fn decompress_undersized_target() {
        let input = b"a somewhat longer payload to compress";
        let mut compressed = [0u8; 128];
        let n = compress(&mut compressed, input).unwrap();
        let mut out = [0u8; 4];
        let err = decompress(&mut out, &compressed[..n]).unwrap_err();
        assert!(matches!(err, EtfError::DecompressionFailed));
    }

    #[test]
    fn compress_undersized_target() {
        let input = [0xABu8; 256];
        let mut target = [0u8; 4];
        let err = compress(&mut target, &input).unwrap_err();
        assert!(matches!(err, EtfError::CompressionFailed));
    }
}
