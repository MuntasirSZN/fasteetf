use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::limits::Limits;
use crate::visitor::Visitor;

// ── Integers ───────────────────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn visit_int<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
) -> Result<(), V::Error> {
    visitor.visit_int(cursor.read_u32()? as i32)
}

// ── Bignums ────────────────────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn visit_small_big<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u8()? as usize;
    if len > limits.max_binary_size {
        return Err(EtfError::BinaryTooLarge.into());
    }
    let sign = cursor.read_u8()?;
    let digits = cursor.take(len)?;
    visitor.visit_big_int(sign, digits)
}

pub(crate) fn visit_large_big<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u32()? as usize;
    if len > limits.max_binary_size {
        return Err(EtfError::BinaryTooLarge.into());
    }
    let sign = cursor.read_u8()?;
    let digits = cursor.take(len)?;
    visitor.visit_big_int(sign, digits)
}

// ── Floats ─────────────────────────────────────────────────────────────────

#[inline(always)]
pub(crate) fn visit_new_float<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
) -> Result<(), V::Error> {
    visitor.visit_float(cursor.read_f64()?)
}

pub(crate) fn visit_legacy_float<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
) -> Result<(), V::Error> {
    let bytes = cursor.take(31)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(31);
    let s = core::str::from_utf8(&bytes[..end]).map_err(|_| EtfError::InvalidUtf8)?;
    let f = s
        .trim_end()
        .parse::<f64>()
        .map_err(|_| EtfError::InvalidFloat)?;
    visitor.visit_float(f)
}

// ── Binaries ───────────────────────────────────────────────────────────────

pub(crate) fn visit_binary<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u32()? as usize;
    if len > limits.max_binary_size {
        return Err(EtfError::BinaryTooLarge.into());
    }
    let data = cursor.take(len)?;
    visitor.visit_binary(data)
}

pub(crate) fn visit_bit_binary<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let len = cursor.read_u32()? as usize;
    if len > limits.max_bit_binary_size {
        return Err(EtfError::BinaryTooLarge.into());
    }
    let bits = cursor.read_u8()?;
    let data = cursor.take(len)?;
    visitor.visit_bit_binary(bits, data)
}
