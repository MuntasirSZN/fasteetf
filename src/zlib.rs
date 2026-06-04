// ──────────────────────────────────────────────────────────────────────────
// Zlib backend
//
// ETF's [`COMPRESSED`] term (tag 80) wraps a zlib stream around a fully
// encoded inner term. Both decoding (during parsing) and encoding (via
// [`encode_to_compressed`](crate::encode_to_compressed)) need a zlib
// implementation. `fasteetf` supports several, selectable at compile
// time via Cargo features or at runtime via a function pointer.
//
// # Compile-time backends (mutually exclusive)
//
// | Feature           | Backend                                                       | Needs `alloc`? |
// |-------------------|---------------------------------------------------------------|----------------|
// | `zlib-rs`         | [`zlib_rs`](https://crates.io/crates/zlib-rs) (default)       | yes (compress only)  |
// | `miniz_oxide`     | [`miniz_oxide`](https://crates.io/crates/miniz_oxide)         | yes (compress only)  |
// | `zlib`            | system zlib via [`libz-sys`](https://crates.io/crates/libz-sys)         | no           |
// | `zlib-default`    | system zlib via `libz-sys` with `libz-sys/default`             | no           |
// | `zlib-ng-compat`  | zlib-ng in compat mode via `libz-sys/zlib-ng`                  | no           |
// | `zlib-ng`         | zlib-ng via [`libz-ng-sys`](https://crates.io/crates/libz-ng-sys)       | no           |
// | `cloudflare-zlib` | Cloudflare zlib via [`cloudflare-zlib-sys`](https://crates.io/crates/cloudflare-zlib-sys) | no  |
//
// Decompression is always available (the pure-Rust backends use a
// stack-allocated state in the decompression path).  Compression
// requires the global allocator for the pure-Rust backends because
// they heap-allocate their internal `CompressorOxide` / `z_stream` /
// `Box<HuffmanOxide>` state; this is what our `alloc` feature
// propagates to `rust-allocator` / `with-alloc` on those backends.
// The C-based backends do not need an allocator.
//
// Names mirror the corresponding `flate2` features for compatibility.
//
// # Custom backends
//
// Implement [`ZlibBackend`] on your own type and pass
// `<MyBackend as ZlibBackend>::decompress` (or any compatible function
// pointer) through [`ParseOptions::zlib_backend`].  For compression,
// pass any function with the [`ZlibCompressFn`] signature through
// `encode_to_compressed`.  A runtime backend, if supplied, takes
// precedence over the compile-time selection.
//
// If no backend is selected at compile time and no runtime backend is
// supplied, encountering a [`COMPRESSED`] term yields
// [`EtfError::UnsupportedTag`].
// ──────────────────────────────────────────────────────────────────────────

use crate::error::EtfError;

/// A zlib decompression backend.
///
/// The default backend is selected at compile time via the `zlib-rs`,
/// `miniz_oxide`, `zlib`, `zlib-default`, `zlib-ng-compat`, `zlib-ng`, or
/// `cloudflare-zlib` Cargo feature. To plug in a custom implementation,
/// implement this trait and pass the static dispatch function through
/// [`ParseOptions::zlib_backend`].
///
/// # Example
///
/// ```ignore
/// use fasteetf::{EtfError, ParseOptions, ZlibBackend, parse_etf, Limits};
/// use core::mem::MaybeUninit;
///
/// struct MyBackend;
/// impl ZlibBackend for MyBackend {
///     fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
///         // ... use your favourite zlib here
///         Ok(())
///     }
/// }
///
/// let mut arena = [MaybeUninit::<u8>::uninit(); 4096];
/// let mut decomp = [0u8; 4096];
/// let opts = ParseOptions {
///     input: compressed,
///     decompressed_buffer: Some(&mut decomp),
///     ast_arena: &mut arena,
///     limits: Limits::default(),
///     zlib_backend: Some(<MyBackend as ZlibBackend>::decompress),
/// };
/// let term = parse_etf(opts)?;
/// # Ok::<_, EtfError>(())
/// ```
pub trait ZlibBackend {
    /// Decompress a zlib-wrapped payload from `input` into `target`.
    ///
    /// `target.len()` is the exact expected size of the uncompressed data,
    /// known a priori from the ETF stream header. Implementations must
    /// consume the entirety of `input` and produce exactly `target.len()`
    /// bytes of output. Any deviation is reported as
    /// [`EtfError::DecompressionFailed`].
    fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError>;
}

// ── Backend implementations ───────────────────────────────────────────────
//
// Each block defines a `decompress` function with the same signature:
//     fn(&mut [u8], &[u8]) -> Result<(), EtfError>
//
// Cargo features are additive: if the user enables more than one `zlib-*`
// feature, the **last one in source order** below wins.  This is the same
// convention that `flate2` uses for its overlapping backends.  The
// ordering is (lowest to highest priority):
//
//   1. `zlib-rs`        — pure-Rust, no system deps (default)
//   2. `miniz_oxide`    — pure-Rust, no system deps
//   3. `zlib`           — system zlib via libz-sys
//   4. `zlib-default`   — system zlib via libz-sys with default features
//   5. `zlib-ng-compat` — zlib-ng in compat mode via libz-sys/zlib-ng
//   6. `zlib-ng`        — native zlib-ng via libz-ng-sys
//   7. `cloudflare-zlib` — Cloudflare's zlib via cloudflare-zlib-sys
//
// A user who wants their own implementation can leave all of the above
// off and supply a backend at runtime via `ParseOptions::zlib_backend`.
//

#[cfg(feature = "zlib-rs")]
mod zlib_rs_impl {
    use super::EtfError;

    #[inline]
    pub fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
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
    pub fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
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
}

#[cfg(feature = "miniz_oxide")]
mod miniz_oxide_impl {
    use super::EtfError;

    #[inline]
    pub fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
        use ::miniz_oxide::inflate::stream::InflateState;
        use ::miniz_oxide::{DataFormat, MZFlush, MZStatus};

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

            let res = ::miniz_oxide::inflate::stream::inflate(
                &mut state,
                in_slice,
                out_slice,
                MZFlush::None,
            );

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
    pub fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
        use ::miniz_oxide::deflate::core::CompressorOxide;
        use ::miniz_oxide::deflate::stream::deflate;
        use ::miniz_oxide::{MZFlush, MZStatus};

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
}

// All C-based backends share the same C `uncompress` calling convention.
// `z_size` is `c_ulong` for libz-sys in zlib/zlib-default/zlib-ng-compat
// modes and for cloudflare-zlib-sys; libz-ng-sys uses `usize` natively.
// We unify them by casting to `c_ulong` (always representable) for the
// libz-sys + cloudflare path and by passing `usize` through directly
// for libz-ng-sys.
#[cfg(any(feature = "zlib", feature = "zlib-default", feature = "zlib-ng-compat"))]
mod libz_sys_impl {
    use super::EtfError;
    use core::ffi::{c_int, c_ulong};

    #[inline]
    pub fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
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
    pub fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
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
}

#[cfg(feature = "zlib-ng")]
mod libz_ng_sys_impl {
    use super::EtfError;
    use core::ffi::c_int;

    #[inline]
    pub fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
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
    pub fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
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
}

#[cfg(feature = "cloudflare-zlib")]
mod cloudflare_zlib_impl {
    use super::EtfError;
    use core::ffi::{c_int, c_ulong};

    #[inline]
    pub fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
        let mut out_len: c_ulong = target.len() as c_ulong;
        let rc: c_int = unsafe {
            ::cloudflare_zlib_sys::uncompress(
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

    const Z_DEFAULT_COMPRESSION: c_int = 6;

    #[inline]
    pub fn compress(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
        let mut out_len: c_ulong = target.len() as c_ulong;
        let rc: c_int = unsafe {
            ::cloudflare_zlib_sys::compress2(
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
}

// ── Public dispatch ──────────────────────────────────────────────────────
//
// `decompress` is called by the parser whenever a `COMPRESSED` wrapper is
// encountered.  It first honours any runtime backend supplied through
// `ParseOptions::zlib_backend`; if none is present, it falls back to the
// compile-time selected backend.  If neither is available (no `zlib-*`
// feature and no runtime override), it returns `UnsupportedTag` so the
// caller can distinguish "compression requested but no backend" from
// "decompression produced bad data".

/// Function pointer type for user-supplied zlib backends.
///
/// A function with this signature can be passed through
/// [`ParseOptions::zlib_backend`](crate::ParseOptions::zlib_backend) to
/// override the compile-time backend at runtime.
pub type ZlibDecompressFn = fn(&mut [u8], &[u8]) -> Result<(), EtfError>;

#[inline]
#[allow(dead_code, unused_assignments)] // unused when no `zlib-*` feature is enabled
pub(crate) fn decompress(
    target: &mut [u8],
    input: &[u8],
    runtime: Option<ZlibDecompressFn>,
) -> Result<(), EtfError> {
    // A user-supplied runtime backend always wins.
    if let Some(backend) = runtime {
        return backend(target, input);
    }

    // Compile-time selection.  The features are additive, so when more
    // than one `zlib-*` feature is enabled, the **last** one in source
    // order wins.  This mirrors `flate2`'s dispatch.
    //
    // The order below matches the priority list in the module doc.
    #[cfg(feature = "zlib-rs")]
    #[allow(unused_variables)]
    let result = zlib_rs_impl::decompress(target, input);

    #[cfg(feature = "miniz_oxide")]
    #[allow(unused_variables)]
    let result = miniz_oxide_impl::decompress(target, input);

    #[cfg(any(feature = "zlib", feature = "zlib-default", feature = "zlib-ng-compat"))]
    #[allow(unused_variables)]
    let result = libz_sys_impl::decompress(target, input);

    #[cfg(feature = "zlib-ng")]
    #[allow(unused_variables)]
    let result = libz_ng_sys_impl::decompress(target, input);

    #[cfg(feature = "cloudflare-zlib")]
    #[allow(unused_variables)]
    let result = cloudflare_zlib_impl::decompress(target, input);

    // Fall-through: no `zlib-*` feature was enabled.  The COMPRESSED
    // wrapper is reported as unsupported so callers can distinguish
    // "no backend" from "backend failed on bad data".
    #[cfg(not(any(
        feature = "zlib-rs",
        feature = "miniz_oxide",
        feature = "zlib",
        feature = "zlib-default",
        feature = "zlib-ng-compat",
        feature = "zlib-ng",
        feature = "cloudflare-zlib",
    )))]
    #[allow(unused_variables)]
    let result = Err(EtfError::UnsupportedTag(crate::tags::COMPRESSED));

    result
}

/// Function pointer type for user-supplied zlib **compression** backends.
///
/// Mirrors [`ZlibDecompressFn`]: a function with this signature can be
/// passed through [`encode_to_compressed`]'s `runtime` argument to
/// override the compile-time backend at runtime.
///
/// The function takes a pre-allocated `target` buffer and an `input`
/// slice, and returns the number of compressed bytes written into
/// `target` (which may be less than `target.len()`).  If the output
/// buffer is too small, the function returns
/// [`EtfError::CompressionFailed`]; the caller should size `target`
/// using the backend's `compress_bound` (zlib-rs: `compress_bound`).
///
/// [`encode_to_compressed`]: crate::encode_to_compressed
pub type ZlibCompressFn = fn(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError>;

/// Compile-time / runtime zlib compression dispatch.
///
/// `runtime`, if `Some`, is always used.  If `None`, the compile-time
/// selected backend (via `zlib-*` features) is used.  If no backend is
/// available at all, returns [`EtfError::CompressionFailed`] so the
/// caller can distinguish "compression requested but no backend" from
/// "backend failed on bad data".
#[inline]
#[allow(dead_code, unused_assignments)] // unused when no `zlib-*` feature is enabled
pub(crate) fn compress(
    target: &mut [u8],
    input: &[u8],
    runtime: Option<ZlibCompressFn>,
) -> Result<usize, EtfError> {
    if let Some(backend) = runtime {
        return backend(target, input);
    }

    // Pure-Rust backends need an allocator for their internal state, so
    // their `compress` impl is itself `#[cfg(feature = "alloc")]`.  The
    // C-based backends are unconditional.
    #[cfg(all(feature = "zlib-rs", feature = "alloc"))]
    #[allow(unused_variables)]
    let result = zlib_rs_impl::compress(target, input);

    #[cfg(all(feature = "miniz_oxide", feature = "alloc"))]
    #[allow(unused_variables)]
    let result = miniz_oxide_impl::compress(target, input);

    #[cfg(any(feature = "zlib", feature = "zlib-default", feature = "zlib-ng-compat"))]
    #[allow(unused_variables)]
    let result = libz_sys_impl::compress(target, input);

    #[cfg(feature = "zlib-ng")]
    #[allow(unused_variables)]
    let result = libz_ng_sys_impl::compress(target, input);

    #[cfg(feature = "cloudflare-zlib")]
    #[allow(unused_variables)]
    let result = cloudflare_zlib_impl::compress(target, input);

    // Fall-through: no `zlib-*` feature is enabled, or the only enabled
    // backends are pure-Rust and our `alloc` is off (so they have no
    // allocator configured).
    #[cfg(not(any(
        all(feature = "zlib-rs", feature = "alloc"),
        all(feature = "miniz_oxide", feature = "alloc"),
        feature = "zlib",
        feature = "zlib-default",
        feature = "zlib-ng-compat",
        feature = "zlib-ng",
        feature = "cloudflare-zlib",
    )))]
    #[allow(unused_variables)]
    let result: Result<usize, EtfError> = Err(EtfError::CompressionFailed);

    result
}
