use crate::error::EtfError;
use core::ffi::c_int;

#[inline]
pub(crate) fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    // libz-ng-sys uses native `usize` for the size arguments.
    let mut out_len: usize = target.len();
    let rc: c_int = unsafe {
        ::libz_ng_sys::uncompress(
            target.as_mut_ptr(),
            &mut out_len,
            input.as_ptr(),
            input.len(),
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
    // libz-ng-sys uses native `usize` for the size arguments.
    let mut out_len: usize = target.len();
    let rc: c_int = unsafe {
        ::libz_ng_sys::compress2(
            target.as_mut_ptr(),
            &mut out_len,
            input.as_ptr(),
            input.len(),
            Z_DEFAULT_COMPRESSION,
        )
    };
    if rc != 0 {
        return Err(EtfError::CompressionFailed);
    }
    Ok(out_len)
}
