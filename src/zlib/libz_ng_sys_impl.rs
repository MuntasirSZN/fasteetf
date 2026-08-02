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

#[cfg(test)]
#[cfg(not(miri))]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let input = b"hello libz-ng roundtrip payload";
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
