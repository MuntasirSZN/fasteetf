use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::types::Term;

// ── Integers ───────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_small_big<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u8()? as usize;
    if len > arena.limits().max_bignum_size {
        return Err(EtfError::BinaryTooLarge);
    }
    let sign = cursor.read_u8()?;
    let digits = cursor.take(len)?;
    Ok(Term::BigInt { sign, digits })
}

#[inline]
pub(crate) fn parse_large_big<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_bignum_size {
        return Err(EtfError::BinaryTooLarge);
    }
    let sign = cursor.read_u8()?;
    let digits = cursor.take(len)?;
    Ok(Term::BigInt { sign, digits })
}

// ── Floats ─────────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_legacy_float<'a>(
    cursor: &mut Cursor<'a>,
    _arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let bytes = cursor.take(31)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(31);
    let s = core::str::from_utf8(&bytes[..end]).map_err(|_| EtfError::InvalidUtf8)?;
    let f = s
        .trim_end()
        .parse::<f64>()
        .map_err(|_| EtfError::InvalidFloat)?;
    Ok(Term::Float(f))
}
