use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::{AtomUtf8, Term};

// ── Atoms (lazy UTF-8 — bytes stored, validated on demand) ─────────────────

#[inline]
pub(crate) fn parse_atom_utf8<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_atom_len {
        return Err(EtfError::AtomTooLarge);
    }
    let bytes = cursor.take(len)?;
    Ok(Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(bytes) }))
}

/// Parse an atom without recursing into arbitrary terms.
///
/// This is used for fields that MUST be atoms (node, module, function names)
/// to prevent resource exhaustion attacks where a crafted payload substitutes
/// a deeply nested term for an atom field.
#[inline]
pub(crate) fn parse_atom_only<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
) -> Result<AtomUtf8<'a>, EtfError> {
    let tag = cursor.read_u8()?;
    match tag {
        ATOM_UTF8_EXT => {
            let len = cursor.read_u16()? as usize;
            if len > arena.limits().max_atom_len {
                return Err(EtfError::AtomTooLarge);
            }
            let bytes = cursor.take(len)?;
            Ok(unsafe { AtomUtf8::from_bytes_unchecked(bytes) })
        }
        SMALL_ATOM_UTF8_EXT => {
            let len = cursor.read_u8()? as usize;
            if len > arena.limits().max_atom_len {
                return Err(EtfError::AtomTooLarge);
            }
            let bytes = cursor.take(len)?;
            Ok(unsafe { AtomUtf8::from_bytes_unchecked(bytes) })
        }
        _ => Err(EtfError::InvalidAtomField),
    }
}

#[inline]
pub(crate) fn parse_small_atom_utf8<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u8()? as usize;
    if len > arena.limits().max_atom_len {
        return Err(EtfError::AtomTooLarge);
    }
    let bytes = cursor.take(len)?;
    Ok(Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(bytes) }))
}
