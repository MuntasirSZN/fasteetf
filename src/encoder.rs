// ─────────────────────────────────────────────────────────────────────────────
// Encoder — Erlang External Term Format (ETF)
//
// Source: https://www.erlang.org/doc/apps/erts/erl_ext_dist
//
// Mirrors the structure of the parser module: one small encoding function
// per tag or tag family, dispatched from a central `encode_term`.
//
// DESIGN NOTES
//
//   * For integer encoding, `SMALL_INTEGER_EXT` is preferred when the value
//     fits in 0–255 because it is the most compact form.
//
//   * Atoms: `SMALL_ATOM_UTF8_EXT` is used for names < 256 bytes;
//     `ATOM_UTF8_EXT` for longer names (up to 65535 bytes).
//
//   * Tuples: `SMALL_TUPLE_EXT` for arity < 256; `LARGE_TUPLE_EXT` for
//     larger arities.
//
//   * Opaque wrappers (Pid, Port, Reference, Function) carry their tag
//     byte so they can be re-emitted exactly as they were parsed.
// ─────────────────────────────────────────────────────────────────────────────

use crate::ETF_MAGIC;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::Term;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

// ── Write cursor ───────────────────────────────────────────────────────────

/// A cursor-based writer that fills a mutable byte buffer.
///
/// Tracks the write offset so callers know how many bytes were produced.
struct Encoder<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> Encoder<'a> {
    #[inline(always)]
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, offset: 0 }
    }

    #[inline(always)]
    fn remaining(&self) -> usize {
        self.buf.len() - self.offset
    }

    #[inline(always)]
    fn reserve(&mut self, n: usize) -> Result<&mut [u8], EtfError> {
        if self.remaining() < n {
            return Err(EtfError::UnexpectedEof);
        }
        let slot = &mut self.buf[self.offset..self.offset + n];
        self.offset += n;
        Ok(slot)
    }
}

/// Trait for encoding targets.  This allows the same encoding logic to
/// write to either a fixed buffer ([`Encoder`]) or a growable [`Vec<u8>`]
/// ([`VecEncoder`]), eliminating code duplication.
pub(crate) trait Sink {
    fn write_u8(&mut self, v: u8) -> Result<(), EtfError>;
    fn write_u16(&mut self, v: u16) -> Result<(), EtfError>;
    fn write_u32(&mut self, v: u32) -> Result<(), EtfError>;
    fn write_f64(&mut self, v: f64) -> Result<(), EtfError>;
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EtfError>;
}

impl<'a> Sink for Encoder<'a> {
    #[inline(always)]
    fn write_u8(&mut self, v: u8) -> Result<(), EtfError> {
        self.reserve(1)?[0] = v;
        Ok(())
    }

    #[inline(always)]
    fn write_u16(&mut self, v: u16) -> Result<(), EtfError> {
        self.reserve(2)?.copy_from_slice(&v.to_be_bytes());
        Ok(())
    }

    #[inline(always)]
    fn write_u32(&mut self, v: u32) -> Result<(), EtfError> {
        self.reserve(4)?.copy_from_slice(&v.to_be_bytes());
        Ok(())
    }

    #[inline(always)]
    fn write_f64(&mut self, v: f64) -> Result<(), EtfError> {
        self.reserve(8)?.copy_from_slice(&v.to_be_bytes());
        Ok(())
    }

    #[inline(always)]
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EtfError> {
        self.reserve(bytes.len())?.copy_from_slice(bytes);
        Ok(())
    }
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Encode a [`Term`] into a pre-allocated buffer.
///
/// Returns the number of bytes written.  The output starts with the ETF
/// magic byte `131` followed by the encoded term.
///
/// # Errors
///
/// Returns [`EtfError::UnexpectedEof`] if `buf` is too small.
pub fn encode_to_buf(term: &Term<'_>, buf: &mut [u8]) -> Result<usize, EtfError> {
    let mut enc = Encoder::new(buf);
    enc.write_u8(ETF_MAGIC)?;
    encode_term(&mut enc, term)?;
    Ok(enc.offset)
}

/// Encode a [`Term`] into a newly allocated `Vec<u8>`.
///
/// Requires the `alloc` feature (enabled by default).
#[cfg(feature = "alloc")]
pub fn encode_to_vec(term: &Term<'_>) -> Result<Vec<u8>, EtfError> {
    // Start with a small buffer (8 bytes for ETF magic + simple terms).
    // The Vec will grow if needed, avoiding the double-walk of estimate_size.
    // This is faster for small terms and only slightly slower for large terms
    // (one potential reallocation vs always walking the tree twice).
    let mut enc = VecEncoder::with_capacity(64);
    enc.write_u8(ETF_MAGIC)?;
    encode_term(&mut enc, term)?;
    Ok(enc.into_vec())
}

/// Encode a [`Term`] into a COMPRESSED-wrapped ETF byte stream.
///
/// The wire format written to `output` is:
///
/// ```text
/// 131 COMPRESSED UncompressedSize ZlibData
/// ```
///
/// where `UncompressedSize` is the 4-byte big-endian byte length of
/// the encoded term (the contents of `intermediate` *after* the leading
/// magic byte — i.e. everything `encode_to_buf` would have written
/// past its first byte), and `ZlibData` is the result of zlib-compressing
/// that same payload.
///
/// # Parameters
///
/// * `term` — the term to encode.
/// * `intermediate` — a scratch buffer to hold the bare ETF bytes
///   (no COMPRESSED wrapper).  The magic byte `131` is written to
///   `intermediate[0]`; the rest of the encoded term follows.  Size
///   it generously — start with 4–8 KiB for typical messages.
/// * `output` — the destination buffer for the COMPRESSED byte stream.
///   Must hold at least `6 + compressed_bound(intermediate_used_len)`
///   bytes.  The 6 bytes are: `131`, `COMPRESSED`, and the 4-byte
///   uncompressed-size prefix.
/// * `compress` — an optional runtime zlib backend.  When `Some`, the
///   supplied function is used; when `None`, the compile-time backend
///   (selected via the `zlib-*` feature) is used.  Pass
///   `<MyBackend as ZlibBackend>::compress`-style function pointers to
///   plug in a custom implementation.
///
/// # Returns
///
/// The number of bytes written to `output`.
///
/// # Errors
///
/// * [`EtfError::UnexpectedEof`] if `intermediate` is too small to
///   hold the encoded term, or if `output` is too small to hold the
///   COMPRESSED wrapper.
/// * [`EtfError::CompressionFailed`] if the zlib backend fails (most
///   commonly: the chosen backend has no allocator configured, e.g.
///   a pure-Rust backend without our `alloc` feature).
///
/// Requires the `compression` feature.  Unlike [`encode_to_vec`], this
/// function does not require `alloc`: both buffers are caller-supplied
/// slices.
#[cfg(feature = "compression")]
pub fn encode_to_compressed(
    term: &Term<'_>,
    intermediate: &mut [u8],
    output: &mut [u8],
    compress: Option<crate::zlib::ZlibCompressFn>,
) -> Result<usize, EtfError> {
    // Step 1: encode the term to `intermediate`.  This writes the magic
    // byte (131) at `intermediate[0]` and the term body afterwards.  We
    // skip the magic byte before compression — the COMPRESSED wrapper
    // re-emits it as part of the outer stream.
    let n = encode_to_buf(term, intermediate)?;
    let body = &intermediate[1..n];

    // Step 2: write the COMPRESSED header into `output`.  Layout:
    //   output[0] = 131             (outer magic)
    //   output[1] = COMPRESSED (80) (outer tag)
    //   output[2..6] = body.len()   (uncompressed size, BE u32)
    //   output[6..6+c] = zlib(body) (compressed body)
    if output.len() < 6 {
        return Err(EtfError::UnexpectedEof);
    }
    output[0] = ETF_MAGIC;
    output[1] = COMPRESSED;
    output[2..6].copy_from_slice(&(body.len() as u32).to_be_bytes());

    // Step 3: compress `body` into the tail of `output`.  The dispatch
    // in `zlib::compress` picks a runtime override, otherwise the
    // compile-time backend.
    let compressed_len = crate::zlib::compress(&mut output[6..], body, compress)?;

    Ok(6 + compressed_len)
}

// ── Theme: encode_term dispatch ────────────────────────────────────────────

/// Recursively encode a single ETF term into any [`Sink`].
fn encode_term<S: Sink>(enc: &mut S, term: &Term) -> Result<(), EtfError> {
    match term {
        Term::Int(v) => encode_int(enc, *v),

        Term::BigInt { sign, digits } => {
            if digits.len() > 255 {
                encode_large_big(enc, *sign, digits)
            } else {
                encode_small_big(enc, *sign, digits)
            }
        }

        Term::Float(v) => encode_float(enc, *v),

        Term::Atom(a) => encode_atom(enc, a.as_bytes()),

        Term::Tuple(elements) => encode_tuple(enc, elements),

        Term::List(elements) => encode_list(enc, elements),
        Term::ImproperList(elements) => {
            // elements slice includes the tail as the last element
            let len = elements.len();
            if len < 2 {
                return Err(EtfError::InvalidSize);
            }
            let (prefix, tail) = elements.split_at(len - 1);
            encode_improper_list(enc, prefix, &tail[0])
        }

        Term::Map(pairs) => encode_map(enc, pairs),

        Term::Binary(data) => encode_binary(enc, data),
        Term::BitBinary { bits, data } => encode_bit_binary(enc, *bits, data),
        Term::String(data) => {
            enc.write_u8(STRING_EXT)?;
            enc.write_u16(data.len() as u16)?;
            enc.write_bytes(data)
        }

        Term::Pid(data) => enc.write_bytes(data),
        Term::Port(data) => enc.write_bytes(data),
        Term::Ref(data) => enc.write_bytes(data),
        Term::Function(data) => {
            // data includes the tag byte and any necessary header fields
            // (e.g., for NEW_FUN_EXT, data includes Tag + Size + payload)
            // Just write the data as-is
            enc.write_bytes(data)
        }
        Term::Record(data) => {
            // data includes the tag byte (RECORD_EXT=67)
            // Just write the data as-is
            enc.write_bytes(data)
        }
    }
}

/// Encode an integer using the most compact representation.
///
/// - 0 … 255 → `SMALL_INTEGER_EXT` (3 bytes total with magic)
/// - otherwise → `INTEGER_EXT` (6 bytes total with magic)
fn encode_int<S: Sink>(enc: &mut S, v: i32) -> Result<(), EtfError> {
    if (0..=255).contains(&v) {
        enc.write_u8(SMALL_INTEGER_EXT)?;
        enc.write_u8(v as u8)
    } else {
        enc.write_u8(INTEGER_EXT)?;
        enc.write_u32(v as u32) // two's complement; cast to u32 gives correct BE bytes
    }
}

// ── Bignums ────────────────────────────────────────────────────────────────

/// Encode `SMALL_BIG_EXT` (110): bignum with 1-byte digit count.
///
/// Wire: `110 n Sign d0…d(n-1)` — digits in little-endian base 256.
fn encode_small_big<S: Sink>(enc: &mut S, sign: u8, digits: &[u8]) -> Result<(), EtfError> {
    let len = digits.len();
    if len > 255 {
        // Too many digits for SMALL_BIG_EXT; upgrade to LARGE_BIG_EXT.
        return encode_large_big(enc, sign, digits);
    }
    enc.write_u8(SMALL_BIG_EXT)?;
    enc.write_u8(len as u8)?;
    enc.write_u8(sign)?;
    enc.write_bytes(digits)
}

/// Encode `LARGE_BIG_EXT` (111): bignum with 4-byte digit count.
///
/// Wire: `111 n Sign d0…d(n-1)` — digits in little-endian base 256.
fn encode_large_big<S: Sink>(enc: &mut S, sign: u8, digits: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(LARGE_BIG_EXT)?;
    enc.write_u32(digits.len() as u32)?;
    enc.write_u8(sign)?;
    enc.write_bytes(digits)
}

// ── Floats ─────────────────────────────────────────────────────────────────

/// Encode `NEW_FLOAT_EXT` (70): IEEE 754 binary64.
///
/// Wire: `70 IEEE_float(8)`
fn encode_float<S: Sink>(enc: &mut S, v: f64) -> Result<(), EtfError> {
    enc.write_u8(NEW_FLOAT_EXT)?;
    enc.write_f64(v)
}

// ── Atoms ──────────────────────────────────────────────────────────────────

/// Encode a UTF-8 atom using the most compact representation.
///
/// - len < 256 → `SMALL_ATOM_UTF8_EXT` (119)
/// - len ≤ 65535 → `ATOM_UTF8_EXT` (118)
fn encode_atom<S: Sink>(enc: &mut S, bytes: &[u8]) -> Result<(), EtfError> {
    let len = bytes.len();
    if len < 256 {
        enc.write_u8(SMALL_ATOM_UTF8_EXT)?;
        enc.write_u8(len as u8)?;
    } else {
        enc.write_u8(ATOM_UTF8_EXT)?;
        enc.write_u16(len as u16)?;
    }
    enc.write_bytes(bytes)
}

// ── Tuples ─────────────────────────────────────────────────────────────────

/// Encode a tuple.
///
/// - arity < 256 → `SMALL_TUPLE_EXT` (104)
/// - arity ≥ 256 → `LARGE_TUPLE_EXT` (105)
fn encode_tuple<S: Sink>(enc: &mut S, elements: &[Term]) -> Result<(), EtfError> {
    let arity = elements.len();
    if arity < 256 {
        enc.write_u8(SMALL_TUPLE_EXT)?;
        enc.write_u8(arity as u8)?;
    } else {
        enc.write_u8(LARGE_TUPLE_EXT)?;
        enc.write_u32(arity as u32)?;
    }
    for elem in elements {
        encode_term(enc, elem)?;
    }
    Ok(())
}

// ── Lists ──────────────────────────────────────────────────────────────────

/// Encode a proper list.
///
/// - empty → `NIL_EXT` (106)
/// - non-empty → `LIST_EXT (108) Len(4) Elements Tail(NIL_EXT)`
fn encode_list<S: Sink>(enc: &mut S, elements: &[Term]) -> Result<(), EtfError> {
    if elements.is_empty() {
        return enc.write_u8(NIL_EXT);
    }
    enc.write_u8(LIST_EXT)?;
    enc.write_u32(elements.len() as u32)?;
    for elem in elements {
        encode_term(enc, elem)?;
    }
    enc.write_u8(NIL_EXT) // proper list tail
}

/// Encode an improper list `[a, b | c]`.
///
/// Wire: `LIST_EXT (108) Len(4) Elements Tail`
fn encode_improper_list<S: Sink>(
    enc: &mut S,
    elements: &[Term],
    tail: &Term,
) -> Result<(), EtfError> {
    enc.write_u8(LIST_EXT)?;
    enc.write_u32(elements.len() as u32)?;
    for elem in elements {
        encode_term(enc, elem)?;
    }
    encode_term(enc, tail)
}

// ── Maps ───────────────────────────────────────────────────────────────────

/// Encode `MAP_EXT` (116): key-value pairs with 4-byte arity.
///
/// Wire: `116 Arity(4) K1 V1 … Kn Vn`
fn encode_map<S: Sink>(enc: &mut S, pairs: &[(Term, Term)]) -> Result<(), EtfError> {
    enc.write_u8(MAP_EXT)?;
    enc.write_u32(pairs.len() as u32)?;
    for (key, value) in pairs {
        encode_term(enc, key)?;
        encode_term(enc, value)?;
    }
    Ok(())
}

// ── Binaries ───────────────────────────────────────────────────────────────

/// Encode `BINARY_EXT` (109): raw binary with 4-byte length.
///
/// Wire: `109 Len(4) Data[Len]`
fn encode_binary<S: Sink>(enc: &mut S, data: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(BINARY_EXT)?;
    enc.write_u32(data.len() as u32)?;
    enc.write_bytes(data)
}

/// Encode `BIT_BINARY_EXT` (77): bitstring with 4-byte length + 1-byte bits.
///
/// Wire: `77 Len(4) Bits(1) Data[Len]`
fn encode_bit_binary<S: Sink>(enc: &mut S, bits: u8, data: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(BIT_BINARY_EXT)?;
    enc.write_u32(data.len() as u32)?;
    enc.write_u8(bits)?;
    enc.write_bytes(data)
}

// ── Opaque wrappers ────────────────────────────────────────────────────────

// ── VecEncoder: a growable encoder for the fallback path ───────────────────

/// A growable encoder that writes into a `Vec<u8>`.
///
/// Used as a fallback when the size estimate was too small.
#[cfg(feature = "alloc")]
struct VecEncoder {
    buf: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl Sink for VecEncoder {
    #[inline(always)]
    fn write_u8(&mut self, v: u8) -> Result<(), EtfError> {
        self.buf.push(v);
        Ok(())
    }

    #[inline(always)]
    fn write_u16(&mut self, v: u16) -> Result<(), EtfError> {
        self.buf.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    #[inline(always)]
    fn write_u32(&mut self, v: u32) -> Result<(), EtfError> {
        self.buf.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    #[inline(always)]
    fn write_f64(&mut self, v: f64) -> Result<(), EtfError> {
        self.buf.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    #[inline(always)]
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EtfError> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl VecEncoder {
    fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}
