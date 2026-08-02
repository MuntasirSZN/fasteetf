use super::visit_term;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::limits::Limits;
use crate::visitor::Visitor;

// ── PIDs ───────────────────────────────────────────────────────────────────

pub(crate) fn visit_pid_legacy<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?; // node atom
    let data = cursor.take(9)?;
    visitor.visit_pid(data)
}

pub(crate) fn visit_pid_new<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?;
    let data = cursor.take(12)?;
    visitor.visit_pid(data)
}

// ── Ports ──────────────────────────────────────────────────────────────────

pub(crate) fn visit_port_legacy<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?;
    let data = cursor.take(5)?;
    visitor.visit_port(data)
}

pub(crate) fn visit_port_new<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?;
    let data = cursor.take(8)?;
    visitor.visit_port(data)
}

pub(crate) fn visit_port_v4<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?;
    let data = cursor.take(12)?;
    visitor.visit_port(data)
}

// ── References ─────────────────────────────────────────────────────────────

pub(crate) fn visit_ref_legacy<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u16()? as usize;
    if len > limits.max_reference_words {
        return Err(EtfError::ListTooLarge.into());
    }
    visit_term(cursor, visitor, depth + 1, limits)?;
    let _creation = cursor.read_u8()?;
    let id = cursor.take(len * 4)?;
    visitor.visit_reference(id)
}

pub(crate) fn visit_ref_newer<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u16()? as usize;
    if len > limits.max_reference_words {
        return Err(EtfError::ListTooLarge.into());
    }
    visit_term(cursor, visitor, depth + 1, limits)?;
    let _creation = cursor.read_u32()?;
    let id = cursor.take(len * 4)?;
    visitor.visit_reference(id)
}

// ── Functions ──────────────────────────────────────────────────────────────

pub(crate) fn visit_new_fun<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let size = cursor.read_u32()? as usize;
    let remaining = size.checked_sub(4).ok_or(EtfError::InvalidSize)?;
    if remaining > limits.max_fun_size {
        return Err(EtfError::BinaryTooLarge.into());
    }
    let data = cursor.take(remaining)?;
    visitor.visit_function(data)
}

pub(crate) fn visit_export<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let start = cursor.consumed();
    visit_term(cursor, visitor, depth + 1, limits)?;
    visit_term(cursor, visitor, depth + 1, limits)?;
    visit_term(cursor, visitor, depth + 1, limits)?;
    let end = cursor.consumed();
    visitor.visit_function(cursor.slice_between(start, end))
}

// ── Records ────────────────────────────────────────────────────────────────

pub(crate) fn visit_record<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let start = cursor.consumed();
    let num_fields = cursor.read_u32()? as usize;
    if num_fields > limits.max_map_len {
        return Err(EtfError::MapTooLarge.into());
    }
    let _flags = cursor.read_u8()?;
    visit_term(cursor, visitor, depth + 1, limits)?; // module
    visit_term(cursor, visitor, depth + 1, limits)?; // name
    for _ in 0..num_fields {
        visit_term(cursor, visitor, depth + 1, limits)?; // field names
    }
    for _ in 0..num_fields {
        visit_term(cursor, visitor, depth + 1, limits)?; // field values
    }
    let end = cursor.consumed();
    visitor.visit_record(cursor.slice_between(start, end))
}
