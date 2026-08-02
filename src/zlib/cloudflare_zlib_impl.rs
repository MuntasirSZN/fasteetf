use crate::error::EtfError;
use core::ffi::c_int;

// cloudflare_zlib_sys defines uLong/uLongf as u64 (see its zconf.h).
// We use u64 directly rather than c_ulong, which is u32 on Windows.
#[inline]
pub(crate) fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    let mut out_len: u64 = target.len() as u64;
    let rc: c_int = unsafe {
        ::cloudflare_zlib_sys::uncompress(
            target.as_mut_ptr(),
            &mut out_len,
            input.as_ptr(),
            input.len() as u64,
        )
    };
    if rc != 0 {
        return Err(EtfError::DecompressionFailed);
    }
    Ok(())
}

const Z_DEFAULT_COMPRESSION: c_int = 6;

#[inline]
pub(crate) fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    let mut out_len: u64 = target.len() as u64;
    let rc: c_int = unsafe {
        ::cloudflare_zlib_sys::compress2(
            target.as_mut_ptr(),
            &mut out_len,
            input.as_ptr(),
            input.len() as u64,
            Z_DEFAULT_COMPRESSION,
        )
    };
    if rc != 0 {
        return Err(EtfError::CompressionFailed);
    }
    Ok(out_len as usize)
}
