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
