use super::Sink;
use super::encode_term;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::Term;

// ── Tuples ─────────────────────────────────────────────────────────────────

/// Encode a tuple.
///
/// - arity < 256 → `SMALL_TUPLE_EXT` (104)
/// - arity ≥ 256 → `LARGE_TUPLE_EXT` (105)
pub(crate) fn encode_tuple<S: Sink>(enc: &mut S, elements: &[Term]) -> Result<(), EtfError> {
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
pub(crate) fn encode_list<S: Sink>(enc: &mut S, elements: &[Term]) -> Result<(), EtfError> {
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
pub(crate) fn encode_improper_list<S: Sink>(
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
pub(crate) fn encode_map<S: Sink>(enc: &mut S, pairs: &[(Term, Term)]) -> Result<(), EtfError> {
    enc.write_u8(MAP_EXT)?;
    enc.write_u32(pairs.len() as u32)?;
    for (key, value) in pairs {
        encode_term(enc, key)?;
        encode_term(enc, value)?;
    }
    Ok(())
}
