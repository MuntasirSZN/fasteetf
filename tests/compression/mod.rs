// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for the compression (COMPRESSED tag) code path.
//
// These tests build a zlib-compressed ETF byte sequence by hand and feed it
// into the parser.  The compression backend is intentionally decoupled from
// the one selected at compile time by `fasteetf` — we use `zlib-rs` from
// dev-dependencies to compress the fixtures, and we also exercise the
// [`ZlibBackend`] trait by passing the function pointer through
// `ParseOptions::zlib_backend`.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]
#![cfg(feature = "compression")]

#[path = "../common/mod.rs"]
mod common;
use common::*;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};
use fasteetf::{
    EtfError, Limits, ParseOptions, Term, ZlibBackend, encode_to_compressed, parse_etf,
};

/// Counter shared between the `CountingBackend` impl and the test body
/// that asserts on it.  Bumping it on every call proves the parser
/// dispatched to the user-supplied runtime backend.
static RUNTIME_BACKEND_CALLS: AtomicUsize = AtomicUsize::new(0);

/// A trivial [`ZlibBackend`] that defers to `zlib-rs`'s one-shot API.
///
/// Used to verify that the runtime backend override is honoured by the
/// parser regardless of the `zlib-*` feature compiled in.
struct ZlibRsBackend;

impl ZlibBackend for ZlibRsBackend {
    fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
        let (_, rc) = zlib_rs::decompress_slice(target, input, Default::default());
        if rc != zlib_rs::ReturnCode::Ok {
            return Err(EtfError::DecompressionFailed);
        }
        Ok(())
    }
}

/// A backend that records how many times it is called and otherwise
/// defers to the real zlib-rs implementation.  Used to prove the runtime
/// hook is honoured over any compile-time default.
struct CountingBackend;

impl ZlibBackend for CountingBackend {
    fn decompress(target: &mut [u8], input: &[u8]) -> Result<(), EtfError> {
        RUNTIME_BACKEND_CALLS.fetch_add(1, Ordering::SeqCst);
        ZlibRsBackend::decompress(target, input)
    }
}

/// Compress `input` with zlib, returning a freshly-allocated `Vec<u8>`.
fn compress_zlib(input: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; zlib_rs::compress_bound(input.len())];
    let (compressed, rc) = zlib_rs::compress_slice(&mut buf, input, Default::default());
    assert_eq!(
        rc,
        zlib_rs::ReturnCode::Ok,
        "compress_slice failed (rc = {rc:?})"
    );
    compressed.to_vec()
}

/// Build a COMPRESSED-tagged ETF byte sequence wrapping `inner`.
///
/// The wire format is:
///
/// ```text
/// 131                       magic
/// 80                        COMPRESSED tag
/// <4 bytes BE>              UncompressedSize
/// <zlib stream>             zlib-wrapped `inner` payload
/// ```
fn compressed_etf(inner: &[u8]) -> Vec<u8> {
    let compressed = compress_zlib(inner);
    let mut out = Vec::with_capacity(1 + 1 + 4 + compressed.len());
    out.push(131);
    out.push(0x50); // COMPRESSED tag
    out.extend_from_slice(&(inner.len() as u32).to_be_bytes());
    out.extend_from_slice(&compressed);
    out
}

fn make_arena() -> Vec<MaybeUninit<u8>> {
    vec![MaybeUninit::<u8>::uninit(); 65536]
}
mod decode;
mod encode;
