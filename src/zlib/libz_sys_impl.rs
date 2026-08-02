use crate::error::EtfError;
use core::ffi::{c_int, c_ulong};

#[inline]
pub(crate) fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    // `target.len()` is the *expected* uncompressed size, declared
    // by the ETF stream header.  On success, `uncompress` updates
    // `out_len` to the actual bytes written.  The function returns
    // `Z_OK` (0) on success and one of `Z_*_ERROR` otherwise.
    let mut out_len: c_ulong = target.len() as c_ulong;
    let rc: c_int = unsafe {
        ::libz_sys::uncompress(
            target.as_mut_ptr(),
            &mut out_len,
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
    let mut out_len: c_ulong = target.len() as c_ulong;
    let rc: c_int = unsafe {
        ::libz_sys::compress2(
            target.as_mut_ptr(),
            &mut out_len,
            input.as_ptr(),
            input.len() as c_ulong,
            Z_DEFAULT_COMPRESSION,
        )
    };
    if rc != 0 {
        return Err(EtfError::CompressionFailed);
    }
    Ok(out_len as usize)
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
