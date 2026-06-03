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

    /// Number of unwritten bytes remaining.
    #[inline(always)]
    fn remaining(&self) -> usize {
        self.buf.len() - self.offset
    }

    /// Reserve capacity for `n` bytes, returning a mutable reference to
    /// the reserved region.  The caller must fill exactly `n` bytes.
    #[inline(always)]
    fn reserve(&mut self, n: usize) -> Result<&mut [u8], EtfError> {
        if self.remaining() < n {
            return Err(EtfError::UnexpectedEof);
        }
        let slot = &mut self.buf[self.offset..self.offset + n];
        self.offset += n;
        Ok(slot)
    }

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
        if bytes.is_empty() {
            return Ok(());
        }
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
    // Use a small initial guess; the Vec grows dynamically if needed.
    let cap = estimate_size(term);
    let mut enc = VecEncoder::with_capacity(cap);
    enc.write_u8(ETF_MAGIC)?;
    encode_term_vec(&mut enc, term)?;
    Ok(enc.into_vec())
}

// ── Theme: encode_term dispatch ────────────────────────────────────────────

/// Recursively encode a single ETF term into the encoder.
fn encode_term(enc: &mut Encoder, term: &Term) -> Result<(), EtfError> {
    match term {
        // ===================================================================
        // Integers
        // ===================================================================
        Term::Int(v) => encode_int(enc, *v),

        // ===================================================================
        // Bignums
        // ===================================================================
        Term::SmallBigInt { sign, digits } => encode_small_big(enc, *sign, digits),
        Term::LargeBigInt { sign, digits } => encode_large_big(enc, *sign, digits),

        // ===================================================================
        // Floats
        // ===================================================================
        Term::Float(v) => encode_float(enc, *v),

        // ===================================================================
        // Atoms
        // ===================================================================
        Term::Atom(a) => encode_atom(enc, a.as_bytes()),

        // ===================================================================
        // Tuples
        // ===================================================================
        Term::Tuple(elements) => encode_tuple(enc, elements),

        // ===================================================================
        // Lists / Nil / Strings
        // ===================================================================
        Term::List(elements) => encode_list(enc, elements),
        Term::ImproperList { elements, tail } => encode_improper_list(enc, elements, tail),

        // ===================================================================
        // Maps
        // ===================================================================
        Term::Map(pairs) => encode_map(enc, pairs),

        // ===================================================================
        // Binaries
        // ===================================================================
        Term::Binary(data) => encode_binary(enc, data),
        Term::BitBinary { bits, data } => encode_bit_binary(enc, *bits, data),

        // ===================================================================
        // Opaque wrappers
        // ===================================================================
        Term::Pid(p) => encode_opaque(enc, p.0, p.1),
        Term::Port(p) => encode_opaque(enc, p.0, p.1),
        Term::Ref(r) => encode_opaque(enc, r.0, r.1),
        Term::Function(f) => {
            // NEW_FUN_EXT stores everything after the Size field; we need
            // to re-insert Size = 4 + data.len().
            if f.0 == NEW_FUN_EXT {
                enc.write_u8(NEW_FUN_EXT)?;
                enc.write_u32(4u32.wrapping_add(f.1.len() as u32))?;
                enc.write_bytes(f.1)?;
            } else {
                encode_opaque(enc, f.0, f.1)?;
            }
            Ok(())
        }
        Term::Record(r) => {
            enc.write_u8(RECORD_EXT)?;
            enc.write_bytes(r.0)?;
            Ok(())
        }
    }
}

// ── Integers ───────────────────────────────────────────────────────────────

/// Encode an integer using the most compact representation.
///
/// - 0 … 255 → `SMALL_INTEGER_EXT` (3 bytes total with magic)
/// - otherwise → `INTEGER_EXT` (6 bytes total with magic)
fn encode_int(enc: &mut Encoder, v: i32) -> Result<(), EtfError> {
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
fn encode_small_big(enc: &mut Encoder, sign: u8, digits: &[u8]) -> Result<(), EtfError> {
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
fn encode_large_big(enc: &mut Encoder, sign: u8, digits: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(LARGE_BIG_EXT)?;
    enc.write_u32(digits.len() as u32)?;
    enc.write_u8(sign)?;
    enc.write_bytes(digits)
}

// ── Floats ─────────────────────────────────────────────────────────────────

/// Encode `NEW_FLOAT_EXT` (70): IEEE 754 binary64.
///
/// Wire: `70 IEEE_float(8)`
fn encode_float(enc: &mut Encoder, v: f64) -> Result<(), EtfError> {
    enc.write_u8(NEW_FLOAT_EXT)?;
    enc.write_f64(v)
}

// ── Atoms ──────────────────────────────────────────────────────────────────

/// Encode a UTF-8 atom using the most compact representation.
///
/// - len < 256 → `SMALL_ATOM_UTF8_EXT` (119)
/// - len ≤ 65535 → `ATOM_UTF8_EXT` (118)
fn encode_atom(enc: &mut Encoder, bytes: &[u8]) -> Result<(), EtfError> {
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
fn encode_tuple(enc: &mut Encoder, elements: &[Term]) -> Result<(), EtfError> {
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
fn encode_list(enc: &mut Encoder, elements: &[Term]) -> Result<(), EtfError> {
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
fn encode_improper_list(enc: &mut Encoder, elements: &[Term], tail: &Term) -> Result<(), EtfError> {
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
fn encode_map(enc: &mut Encoder, pairs: &[(Term, Term)]) -> Result<(), EtfError> {
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
fn encode_binary(enc: &mut Encoder, data: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(BINARY_EXT)?;
    enc.write_u32(data.len() as u32)?;
    enc.write_bytes(data)
}

/// Encode `BIT_BINARY_EXT` (77): bitstring with 4-byte length + 1-byte bits.
///
/// Wire: `77 Len(4) Bits(1) Data[Len]`
fn encode_bit_binary(enc: &mut Encoder, bits: u8, data: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(BIT_BINARY_EXT)?;
    enc.write_u32(data.len() as u32)?;
    enc.write_u8(bits)?;
    enc.write_bytes(data)
}

// ── Opaque wrappers ────────────────────────────────────────────────────────

/// Encode an opaque wrapper: write the tag byte followed by the raw bytes.
fn encode_opaque(enc: &mut Encoder, tag: u8, data: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(tag)?;
    enc.write_bytes(data)
}

// ── Size estimation (allows single-pass Vec allocation) ────────────────────

/// Estimate the encoded size of a term (over-estimate is safe).
///
/// This is used by `encode_to_vec` to pick an initial buffer capacity.
/// Over-estimating is fine; we truncate at the end.
#[cfg(feature = "alloc")]
fn estimate_size(term: &Term) -> usize {
    // Magic byte + per-term overhead.
    1 + match term {
        Term::Int(v) => {
            if (0..=255).contains(v) {
                2 // SMALL_INTEGER_EXT + 1 byte
            } else {
                5 // INTEGER_EXT + 4 bytes
            }
        }
        Term::SmallBigInt { sign: _, digits } => {
            if digits.len() > 255 {
                6 + digits.len() // LARGE_BIG_EXT + 4 + 1 + digits
            } else {
                3 + digits.len() // SMALL_BIG_EXT + 1 + 1 + digits
            }
        }
        Term::LargeBigInt { sign: _, digits } => 6 + digits.len(),
        Term::Float(_) => 9, // NEW_FLOAT_EXT + 8 bytes
        Term::Atom(a) => {
            let len = a.len();
            if len < 256 {
                2 + len // SMALL_ATOM_UTF8_EXT + 1 + data
            } else {
                3 + len // ATOM_UTF8_EXT + 2 + data
            }
        }
        Term::Tuple(elements) => {
            let arity = elements.len();
            let header = if arity < 256 { 2 } else { 5 };
            header + elements.iter().map(estimate_size).sum::<usize>()
        }
        Term::List(elements) => {
            if elements.is_empty() {
                1 // NIL_EXT
            } else {
                5 + elements.iter().map(estimate_size).sum::<usize>() + 1 // LIST_EXT + 4 + elements + NIL
            }
        }
        Term::ImproperList { elements, tail } => {
            5 + elements.iter().map(estimate_size).sum::<usize>() + estimate_size(tail)
        }
        Term::Map(pairs) => {
            5 + pairs
                .iter()
                .map(|(k, v)| estimate_size(k) + estimate_size(v))
                .sum::<usize>()
        }
        Term::Binary(data) => 5 + data.len(),
        Term::BitBinary { bits: _, data } => 6 + data.len(),
        Term::Pid(p) => 1 + p.1.len(),
        Term::Port(p) => 1 + p.1.len(),
        Term::Ref(r) => 1 + r.1.len(),
        Term::Function(f) => {
            if f.0 == NEW_FUN_EXT {
                5 + f.1.len() // tag + Size(4) + data
            } else {
                1 + f.1.len()
            }
        }
        Term::Record(r) => 1 + r.0.len(),
    }
}

// ── VecEncoder: a growable encoder for the fallback path ───────────────────

/// A growable encoder that writes into a `Vec<u8>`.
///
/// Used as a fallback when the size estimate was too small.
#[cfg(feature = "alloc")]
struct VecEncoder {
    buf: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl VecEncoder {
    fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    fn write_u8(&mut self, v: u8) -> Result<(), EtfError> {
        self.buf.push(v);
        Ok(())
    }

    fn write_u16(&mut self, v: u16) -> Result<(), EtfError> {
        self.buf.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn write_u32(&mut self, v: u32) -> Result<(), EtfError> {
        self.buf.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn write_f64(&mut self, v: f64) -> Result<(), EtfError> {
        self.buf.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EtfError> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }

    fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

/// Encode a term into a `VecEncoder` (fallback path when size estimate
/// was too small).
#[cfg(feature = "alloc")]
fn encode_term_vec(enc: &mut VecEncoder, term: &Term) -> Result<(), EtfError> {
    match term {
        Term::Int(v) => {
            if (0..=255).contains(v) {
                enc.write_u8(SMALL_INTEGER_EXT)?;
                enc.write_u8(*v as u8)
            } else {
                enc.write_u8(INTEGER_EXT)?;
                enc.write_u32(*v as u32)
            }
        }
        Term::SmallBigInt { sign, digits } => {
            if digits.len() > 255 {
                enc.write_u8(LARGE_BIG_EXT)?;
                enc.write_u32(digits.len() as u32)?;
            } else {
                enc.write_u8(SMALL_BIG_EXT)?;
                enc.write_u8(digits.len() as u8)?;
            }
            enc.write_u8(*sign)?;
            enc.write_bytes(digits)
        }
        Term::LargeBigInt { sign, digits } => {
            enc.write_u8(LARGE_BIG_EXT)?;
            enc.write_u32(digits.len() as u32)?;
            enc.write_u8(*sign)?;
            enc.write_bytes(digits)
        }
        Term::Float(v) => {
            enc.write_u8(NEW_FLOAT_EXT)?;
            enc.write_f64(*v)
        }
        Term::Atom(a) => {
            let bytes = a.as_bytes();
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
        Term::Tuple(elements) => {
            let arity = elements.len();
            if arity < 256 {
                enc.write_u8(SMALL_TUPLE_EXT)?;
                enc.write_u8(arity as u8)?;
            } else {
                enc.write_u8(LARGE_TUPLE_EXT)?;
                enc.write_u32(arity as u32)?;
            }
            for elem in elements.iter() {
                encode_term_vec(enc, elem)?;
            }
            Ok(())
        }
        Term::List(elements) => {
            if elements.is_empty() {
                return enc.write_u8(NIL_EXT);
            }
            enc.write_u8(LIST_EXT)?;
            enc.write_u32(elements.len() as u32)?;
            for elem in elements.iter() {
                encode_term_vec(enc, elem)?;
            }
            enc.write_u8(NIL_EXT)
        }
        Term::ImproperList { elements, tail } => {
            enc.write_u8(LIST_EXT)?;
            enc.write_u32(elements.len() as u32)?;
            for elem in elements.iter() {
                encode_term_vec(enc, elem)?;
            }
            encode_term_vec(enc, tail)
        }
        Term::Map(pairs) => {
            enc.write_u8(MAP_EXT)?;
            enc.write_u32(pairs.len() as u32)?;
            for (key, value) in pairs.iter() {
                encode_term_vec(enc, key)?;
                encode_term_vec(enc, value)?;
            }
            Ok(())
        }
        Term::Binary(data) => {
            enc.write_u8(BINARY_EXT)?;
            enc.write_u32(data.len() as u32)?;
            enc.write_bytes(data)
        }
        Term::BitBinary { bits, data } => {
            enc.write_u8(BIT_BINARY_EXT)?;
            enc.write_u32(data.len() as u32)?;
            enc.write_u8(*bits)?;
            enc.write_bytes(data)
        }
        Term::Pid(p) => {
            enc.write_u8(p.0)?;
            enc.write_bytes(p.1)
        }
        Term::Port(p) => {
            enc.write_u8(p.0)?;
            enc.write_bytes(p.1)
        }
        Term::Ref(r) => {
            enc.write_u8(r.0)?;
            enc.write_bytes(r.1)
        }
        Term::Function(f) => {
            if f.0 == NEW_FUN_EXT {
                enc.write_u8(NEW_FUN_EXT)?;
                enc.write_u32(4u32.wrapping_add(f.1.len() as u32))?;
                enc.write_bytes(f.1)?;
            } else {
                enc.write_u8(f.0)?;
                enc.write_bytes(f.1)?;
            }
            Ok(())
        }
        Term::Record(r) => {
            enc.write_u8(RECORD_EXT)?;
            enc.write_bytes(r.0)
        }
    }
}
