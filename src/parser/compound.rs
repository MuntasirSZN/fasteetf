use super::parse_term;
use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::types::Term;

// ── Tuples ─────────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_small_tuple<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    let arity = cursor.read_u8()? as usize;
    parse_tuple_elements(cursor, arena, arity, depth).map(Term::Tuple)
}

#[inline]
pub(crate) fn parse_large_tuple<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    let arity = cursor.read_u32()? as usize;
    parse_tuple_elements(cursor, arena, arity, depth).map(Term::Tuple)
}

/// Shared helper: allocate and recursively parse `arity` elements.
#[inline]
pub(crate) fn parse_tuple_elements<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    arity: usize,
    depth: usize,
) -> Result<&'a [Term<'a>], EtfError> {
    if arity > arena.limits().max_tuple_arity {
        return Err(EtfError::TupleTooLarge);
    }
    let elements = arena.alloc_slice(arity)?;
    for elem in elements.iter_mut() {
        *elem = parse_term(cursor, arena, depth + 1)?;
    }
    Ok(elements)
}
// ── Lists / Strings ────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_string<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
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
pub(crate) fn parse_list<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_list_len {
        return Err(EtfError::ListTooLarge);
    }
    let elements = arena.alloc_slice(len)?;
    for elem in elements.iter_mut() {
        *elem = parse_term(cursor, arena, depth + 1)?;
    }

    let tail = parse_term(cursor, arena, depth + 1)?;
    match tail {
        Term::List([]) => Ok(Term::List(elements)),
        _ => {
            // New representation: single slice with tail as last element
            let total_len = len + 1;
            let all_elements = arena.alloc_slice(total_len)?;
            all_elements[..len].copy_from_slice(elements);
            all_elements[len] = tail;
            Ok(Term::ImproperList(all_elements))
        }
    }
}

// ── Maps ───────────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_map<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_map_len {
        return Err(EtfError::MapTooLarge);
    }
    let pairs = arena.alloc_slice::<(Term<'a>, Term<'a>)>(len)?;
    for pair in pairs.iter_mut() {
        let key = parse_term(cursor, arena, depth + 1)?;
        let value = parse_term(cursor, arena, depth + 1)?;
        *pair = (key, value);
    }
    Ok(Term::Map(pairs))
}

// ── Binaries ───────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_binary<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
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
    _depth: usize,
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
