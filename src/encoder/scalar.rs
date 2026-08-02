use super::Sink;
use crate::error::EtfError;
use crate::tags::*;

// ── Integers ─────────────────────────────────────────────────────────────────

/// Encode an integer using the most compact representation.
///
/// - 0 … 255 → `SMALL_INTEGER_EXT` (3 bytes total with magic)
/// - otherwise → `INTEGER_EXT` (6 bytes total with magic)
pub(crate) fn encode_int<S: Sink>(enc: &mut S, v: i32) -> Result<(), EtfError> {
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
pub(crate) fn encode_small_big<S: Sink>(
    enc: &mut S,
    sign: u8,
    digits: &[u8],
) -> Result<(), EtfError> {
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
pub(crate) fn encode_large_big<S: Sink>(
    enc: &mut S,
    sign: u8,
    digits: &[u8],
) -> Result<(), EtfError> {
    enc.write_u8(LARGE_BIG_EXT)?;
    enc.write_u32(digits.len() as u32)?;
    enc.write_u8(sign)?;
    enc.write_bytes(digits)
}

// ── Floats ─────────────────────────────────────────────────────────────────

/// Encode `NEW_FLOAT_EXT` (70): IEEE 754 binary64.
///
/// Wire: `70 IEEE_float(8)`
pub(crate) fn encode_float<S: Sink>(enc: &mut S, v: f64) -> Result<(), EtfError> {
    enc.write_u8(NEW_FLOAT_EXT)?;
    enc.write_f64(v)
}

// ── Atoms ──────────────────────────────────────────────────────────────────

/// Encode a UTF-8 atom using the most compact representation.
///
/// - len < 256 → `SMALL_ATOM_UTF8_EXT` (119)
/// - len ≤ 65535 → `ATOM_UTF8_EXT` (118)
pub(crate) fn encode_atom<S: Sink>(enc: &mut S, bytes: &[u8]) -> Result<(), EtfError> {
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

// ── Binaries ───────────────────────────────────────────────────────────────

/// Encode `BINARY_EXT` (109): raw binary with 4-byte length.
///
/// Wire: `109 Len(4) Data[Len]`
pub(crate) fn encode_binary<S: Sink>(enc: &mut S, data: &[u8]) -> Result<(), EtfError> {
    enc.write_u8(BINARY_EXT)?;
    enc.write_u32(data.len() as u32)?;
    enc.write_bytes(data)
}

/// Encode `BIT_BINARY_EXT` (77): bitstring with 4-byte length + 1-byte bits.
///
/// Wire: `77 Len(4) Bits(1) Data[Len]`
pub(crate) fn encode_bit_binary<S: Sink>(
    enc: &mut S,
    bits: u8,
    data: &[u8],
) -> Result<(), EtfError> {
    enc.write_u8(BIT_BINARY_EXT)?;
    enc.write_u32(data.len() as u32)?;
    enc.write_u8(bits)?;
    enc.write_bytes(data)
}
