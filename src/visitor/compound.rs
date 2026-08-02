use super::visit_term;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::limits::Limits;
use crate::tags::*;
use crate::visitor::Visitor;

// ── Tuples ─────────────────────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn visit_small_tuple<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let arity = cursor.read_u8()? as usize;
    if arity > limits.max_tuple_arity {
        return Err(EtfError::TupleTooLarge.into());
    }
    visitor.visit_tuple_start(arity)?;
    for _ in 0..arity {
        visit_term(cursor, visitor, depth + 1, limits)?;
    }
    visitor.visit_tuple_end()
}

pub(crate) fn visit_large_tuple<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let arity = cursor.read_u32()? as usize;
    if arity > limits.max_tuple_arity {
        return Err(EtfError::TupleTooLarge.into());
    }
    visitor.visit_tuple_start(arity)?;
    for _ in 0..arity {
        visit_term(cursor, visitor, depth + 1, limits)?;
    }
    visitor.visit_tuple_end()
}

// ── Lists / Nil / Strings ──────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn visit_nil<V: Visitor>(visitor: &mut V) -> Result<(), V::Error> {
    visitor.visit_list_start(0)?;
    visitor.visit_list_end()
}

pub(crate) fn visit_string<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u16()? as usize;
    if len > limits.max_string_len {
        return Err(EtfError::ListTooLarge.into());
    }
    let bytes = cursor.take(len)?;
    visitor.visit_string(bytes)
}

#[inline(always)]
pub(crate) fn visit_list<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u32()? as usize;
    if len > limits.max_list_len {
        return Err(EtfError::ListTooLarge.into());
    }
    visitor.visit_list_start(len)?;
    for _ in 0..len {
        visit_term(cursor, visitor, depth + 1, limits)?;
    }
    // Tail.
    let tail_tag = cursor
        .data
        .first()
        .copied()
        .ok_or(EtfError::UnexpectedEof)?;
    if tail_tag == NIL_EXT {
        cursor.read_u8()?; // consume nil
        visitor.visit_list_end()?;
    } else {
        visitor.visit_improper_list_tail()?;
        visit_term(cursor, visitor, depth + 1, limits)?;
        visitor.visit_improper_list_end()?;
    }
    Ok(())
}

// ── Maps ───────────────────────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn visit_map<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let arity = cursor.read_u32()? as usize;
    if arity > limits.max_map_len {
        return Err(EtfError::MapTooLarge.into());
    }
    visitor.visit_map_start(arity)?;
    for _ in 0..arity {
        visit_term(cursor, visitor, depth + 1, limits)?; // key
        visit_term(cursor, visitor, depth + 1, limits)?; // value
    }
    visitor.visit_map_end()
}
