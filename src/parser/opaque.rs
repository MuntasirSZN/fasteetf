use super::atom::parse_atom_only;
use super::parse_term;
use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::types::Term;

// ── Process identifiers (PIDs) ─────────────────────────────────────────────

#[inline]
pub(crate) fn parse_pid_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(9)?;
    let end = cursor.consumed();
    // Include tag byte in the slice
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Pid(slice))
}

#[inline]
pub(crate) fn parse_pid_new<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(12)?;
    let end = cursor.consumed();
    // Include tag byte in the slice
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Pid(slice))
}

// ── Ports ──────────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_port_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(5)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Port(slice))
}

#[inline]
pub(crate) fn parse_port_new<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(8)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Port(slice))
}

#[inline]
pub(crate) fn parse_port_v4<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(12)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Port(slice))
}

// ── References ─────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_ref_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_reference_words {
        return Err(EtfError::ReferenceTooLarge);
    }
    let _node = parse_atom_only(cursor, arena)?;
    let _creation = cursor.read_u8()?;
    let _ids = cursor.take(len * 4)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Ref(slice))
}

#[inline]
pub(crate) fn parse_ref_newer<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_reference_words {
        return Err(EtfError::ReferenceTooLarge);
    }
    let _node = parse_atom_only(cursor, arena)?;
    let _creation = cursor.read_u32()?;
    let _ids = cursor.take(len * 4)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Ref(slice))
}

// ── Functions ──────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_new_fun<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: usize,
) -> Result<Term<'a>, EtfError> {
    let size = cursor.read_u32()? as usize;
    let remaining = size.checked_sub(4).ok_or(EtfError::InvalidSize)?;
    if remaining > arena.limits().max_fun_size {
        return Err(EtfError::BinaryTooLarge);
    }
    // Include tag byte and Size field in the slice
    // Tag (1 byte) + Size (4 bytes) + payload (remaining bytes)
    let start = cursor.consumed() - 5; // Go back 5 bytes: 1 tag + 4 Size
    let end = start + 5 + remaining; // 5 bytes (tag + Size) + remaining payload
    let slice = cursor.slice_between(start, end);
    Ok(Term::Function(slice))
}

#[inline]
pub(crate) fn parse_export<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed() - 1; // Include tag byte
    let _module = parse_atom_only(cursor, arena)?;
    let _function = parse_atom_only(cursor, arena)?;
    let _arity = parse_term(cursor, arena, depth + 1)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start, end);
    Ok(Term::Function(slice))
}

// ── Records ────────────────────────────────────────────────────────────────

#[inline]
pub(crate) fn parse_record<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed() - 1; // Include tag byte
    let num_fields = cursor.read_u32()? as usize;
    if num_fields > arena.limits().max_map_len {
        return Err(EtfError::MapTooLarge);
    }
    let _flags = cursor.read_u8()?;
    let _module = parse_term(cursor, arena, depth + 1)?;
    let _name = parse_term(cursor, arena, depth + 1)?;
    for _ in 0..num_fields {
        let _ = parse_term(cursor, arena, depth + 1)?;
    }
    for _ in 0..num_fields {
        let _ = parse_term(cursor, arena, depth + 1)?;
    }
    let end = cursor.consumed();
    let slice = cursor.slice_between(start, end);
    Ok(Term::Record(slice))
}
