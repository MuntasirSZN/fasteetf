use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::types::Term;

// ── Strings / Binaries ──────────────────────────────────────────────────────
//
// Tuples, lists, and maps are parsed iteratively by the dispatcher in
// `mod.rs` (see the `Frame` machinery); the helpers below are leaf
// parsers that never recurse.

#[inline]
pub(crate) fn parse_string<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_string_len {
        return Err(EtfError::ListTooLarge);
    }
    let bytes = cursor.take(len)?;

    // STRING_EXT: opt-in compact representation (see Limits::expand_string_ext_to_list)
    if arena.limits().expand_string_ext_to_list {
        // Legacy behavior: expand to List of Ints (default, backward compatible)
        let elements = arena.alloc_slice(len)?;
        for (elem, &b) in elements.iter_mut().zip(bytes.iter()) {
            *elem = Term::Int(b as i32);
        }
        Ok(Term::List(elements))
    } else {
        // Compact behavior: keep as String (zero arena allocation)
        Ok(Term::String(bytes))
    }
}

#[inline]
pub(crate) fn parse_binary<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_binary_size {
        return Err(EtfError::BinaryTooLarge);
    }
    Ok(Term::Binary(cursor.take(len)?))
}

#[inline]
pub(crate) fn parse_bit_binary<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_bit_binary_size {
        return Err(EtfError::BinaryTooLarge);
    }
    let bits = cursor.read_u8()?;
    Ok(Term::BitBinary {
        bits,
        data: cursor.take(len)?,
    })
}
