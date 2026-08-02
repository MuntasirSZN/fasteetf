use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::limits::Limits;
use crate::visitor::Visitor;

// ── Atoms ──────────────────────────────────────────────────────────────────

pub(crate) fn visit_atom_utf8<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u16()? as usize;
    if len > limits.max_atom_len {
        return Err(EtfError::AtomTooLarge.into());
    }
    let bytes = cursor.take(len)?;
    visitor.visit_atom(bytes)
}

pub(crate) fn visit_small_atom_utf8<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u8()? as usize;
    if len > limits.max_atom_len {
        return Err(EtfError::AtomTooLarge.into());
    }
    let bytes = cursor.take(len)?;
    visitor.visit_atom(bytes)
}
