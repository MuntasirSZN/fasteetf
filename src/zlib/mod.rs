// ──────────────────────────────────────────────────────────────────────────
// Zlib backend
//
// ETF's [`COMPRESSED`] term (tag 80) wraps a zlib stream around a fully
// encoded inner term. Both decoding (during parsing) and encoding (via
// [`encode_to_compressed`](crate::encode_to_compressed)) need a zlib
// implementation. `fasteetf` supports several, selectable at compile
// time via Cargo features or at runtime via a function pointer.
//
// # Compile-time backends (additive, last-one-wins)
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
// they heap-allocate their internal `CompressorOxide` / `zlib_z_stream` /
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
//
// # Multiple backends
//
// Enabling more than one zlib backend simultaneously compiles all of them
// in, but only the last one in source order (highest priority) is used at
// runtime.  This wastes compile time and binary size.  Consider enabling
// only one backend.
// ──────────────────────────────────────────────────────────────────────────

use crate::error::EtfError;

/// A zlib decompression backend.
///
/// The default backend is selected at compile time via the `zlib-rs`,
/// `miniz_oxide`, `zlib`, `zlib-default`, `zlib-ng-compat`, `zlib-ng`, or
/// `cloudflare-zlib` Cargo feature. To plug in a custom implementation,
/// implement this trait and pass the static dispatch function through
/// [`ParseOptions`](crate::ParseOptions)::zlib_backend.
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
mod zlib_rs_impl;

#[cfg(feature = "miniz_oxide")]
mod miniz_oxide_impl;

// All C-based backends share the same C `uncompress` calling convention.
// `z_size` is `c_ulong` for libz-sys in zlib/zlib-default/zlib-ng-compat
// modes and for cloudflare-zlib-sys; libz-ng-sys uses `usize` natively.
// We unify them by casting to `c_ulong` (always representable) for the
// libz-sys + cloudflare path and by passing `usize` through directly
// for libz-ng-sys.
#[cfg(any(feature = "zlib", feature = "zlib-default", feature = "zlib-ng-compat"))]
mod libz_sys_impl;

#[cfg(feature = "zlib-ng")]
mod libz_ng_sys_impl;

#[cfg(feature = "cloudflare-zlib")]
mod cloudflare_zlib_impl;

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
/// [`ParseOptions`](crate::ParseOptions)::zlib_backend to
/// override the compile-time backend at runtime.
pub type ZlibDecompressFn = fn(&mut [u8], &[u8]) -> Result<(), EtfError>;

#[cfg(feature = "compression")]
#[inline]
pub(crate) fn decompress(
    target: &mut [u8],
    input: &[u8],
    runtime: Option<ZlibDecompressFn>,
) -> Result<(), EtfError> {
    // A user-supplied runtime backend always wins.
    if let Some(backend) = runtime {
        return backend(target, input);
    }

    decompress_compile_time(target, input)
}

#[cfg(any(
    feature = "zlib-rs",
    feature = "miniz_oxide",
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
#[inline]
#[allow(unreachable_code)]
fn decompress_compile_time(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
    // Compile-time selection.  The features are additive, so when more
    // than one `zlib-*` feature is enabled, the **last** one in source
    // order wins.  This mirrors `flate2`'s dispatch.
    //
    // The order below matches the priority list in the module doc.
    #[cfg(feature = "zlib-rs")]
    return zlib_rs_impl::decompress(target, input);

    #[cfg(feature = "miniz_oxide")]
    return miniz_oxide_impl::decompress(target, input);

    #[cfg(any(feature = "zlib", feature = "zlib-default", feature = "zlib-ng-compat"))]
    return libz_sys_impl::decompress(target, input);

    #[cfg(feature = "zlib-ng")]
    return libz_ng_sys_impl::decompress(target, input);

    #[cfg(feature = "cloudflare-zlib")]
    return cloudflare_zlib_impl::decompress(target, input);
}

#[cfg(not(any(
    feature = "zlib-rs",
    feature = "miniz_oxide",
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
)))]
#[inline]
fn decompress_compile_time(_target: &mut [u8], _input: &[u8]) -> Result<(), EtfError> {
    Err(EtfError::UnsupportedTag(crate::tags::COMPRESSED))
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
#[cfg(feature = "compression")]
pub type ZlibCompressFn = fn(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError>;

#[cfg(feature = "compression")]
#[inline]
pub(crate) fn compress(
    target: &mut [u8],
    input: &[u8],
    runtime: Option<ZlibCompressFn>,
) -> Result<usize, EtfError> {
    if let Some(backend) = runtime {
        return backend(target, input);
    }

    compress_compile_time(target, input)
}

#[cfg(any(
    all(feature = "zlib-rs", feature = "alloc"),
    all(feature = "miniz_oxide", feature = "alloc"),
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
))]
#[inline]
#[allow(unreachable_code)]
fn compress_compile_time(target: &mut [u8], input: &[u8]) -> Result<usize, EtfError> {
    #[cfg(all(feature = "zlib-rs", feature = "alloc"))]
    return zlib_rs_impl::compress(target, input);

    #[cfg(all(feature = "miniz_oxide", feature = "alloc"))]
    return miniz_oxide_impl::compress(target, input);

    #[cfg(any(feature = "zlib", feature = "zlib-default", feature = "zlib-ng-compat"))]
    return libz_sys_impl::compress(target, input);

    #[cfg(feature = "zlib-ng")]
    return libz_ng_sys_impl::compress(target, input);

    #[cfg(feature = "cloudflare-zlib")]
    return cloudflare_zlib_impl::compress(target, input);
}

#[cfg(not(any(
    all(feature = "zlib-rs", feature = "alloc"),
    all(feature = "miniz_oxide", feature = "alloc"),
    feature = "zlib",
    feature = "zlib-default",
    feature = "zlib-ng-compat",
    feature = "zlib-ng",
    feature = "cloudflare-zlib",
)))]
#[inline]
fn compress_compile_time(_target: &mut [u8], _input: &[u8]) -> Result<usize, EtfError> {
    Err(EtfError::CompressionFailed)
}
