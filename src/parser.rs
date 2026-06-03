use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::{AtomUtf8, Function, Pid, Port, Record, Reference, Term};

/// Recursively parse a single ETF term from `cursor`, allocating compound
/// storage from `arena` and enforcing the recursion limit in `depth`.
///
/// ## Fast path
///
/// The small-integer tag (`97`) is checked first because small integer terms
/// are the most frequently encountered term type in typical Erlang messages.
/// This single comparison saves a full match against every tag variant.
///
/// ## Limits
///
/// The recursion budget is passed as a separate `&mut usize` so the
/// compiler can keep it in a register across the inner call instead of
/// clobbering a struct field through `&mut Bump`.  Inside the inner
/// dispatch the same pointer is forwarded as a raw `*mut usize` to
/// prevent the helpers from being treated as potential writers to it
/// — only `parse_term` actually mutates the depth counter.
#[inline(always)]
pub(crate) fn parse_term<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    if *depth == 0 {
        return Err(EtfError::RecursionLimitExceeded);
    }
    *depth -= 1;
    let result = parse_term_inner(cursor, arena, depth);
    *depth += 1;
    result
}

/// The inner dispatch — called once per nesting level by [`parse_term`].
#[inline(always)]
fn parse_term_inner<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let tag = cursor.read_u8()?;

    // ── Fast path ──────────────────────────────────────────────────────
    if tag == SMALL_INTEGER_EXT {
        return Ok(Term::Int(cursor.read_u8()? as i32));
    }

    // ── Second-fastest path: simple tags inlined here ──────────────────
    match tag {
        INTEGER_EXT => return Ok(Term::Int(cursor.read_u32()? as i32)),
        NEW_FLOAT_EXT => return Ok(Term::Float(cursor.read_f64()?)),
        NIL_EXT => return Ok(Term::List(&[])),
        _ => {}
    }

    match tag {
        SMALL_BIG_EXT => parse_small_big(cursor, arena, depth),
        LARGE_BIG_EXT => parse_large_big(cursor, arena, depth),

        FLOAT_EXT => parse_legacy_float(cursor, arena, depth),

        ATOM_UTF8_EXT => parse_atom_utf8(cursor, arena, depth),
        SMALL_ATOM_UTF8_EXT => parse_small_atom_utf8(cursor, arena, depth),

        SMALL_TUPLE_EXT => parse_small_tuple(cursor, arena, depth),
        LARGE_TUPLE_EXT => parse_large_tuple(cursor, arena, depth),

        STRING_EXT => parse_string(cursor, arena, depth),
        LIST_EXT => parse_list(cursor, arena, depth),

        MAP_EXT => parse_map(cursor, arena, depth),

        BINARY_EXT => parse_binary(cursor, arena, depth),
        BIT_BINARY_EXT => parse_bit_binary(cursor, arena, depth),

        PID_EXT => parse_pid_legacy(cursor, arena, depth),
        NEW_PID_EXT => parse_pid_new(cursor, arena, depth),

        PORT_EXT => parse_port_legacy(cursor, arena, depth),
        NEW_PORT_EXT => parse_port_new(cursor, arena, depth),
        V4_PORT_EXT => parse_port_v4(cursor, arena, depth),

        NEW_REFERENCE_EXT => parse_ref_legacy(cursor, arena, depth),
        NEWER_REFERENCE_EXT => parse_ref_newer(cursor, arena, depth),

        NEW_FUN_EXT => parse_new_fun(cursor, arena, depth),
        EXPORT_EXT => parse_export(cursor, arena, depth),

        RECORD_EXT => parse_record(cursor, arena, depth),

        LOCAL_EXT | COMPRESSED | ATOM_CACHE_REF | DIST_HEADER | DIST_HEADER_FRAG_START => {
            Err(EtfError::UnsupportedTag(tag))
        }

        _ => Err(EtfError::UnsupportedTag(tag)),
    }
}

// ── Small specialized parse functions ───────────────────────────────────────
//
// Every function now takes (cursor, arena, depth) — the depth is the
// recursion budget passed by `parse_term`.  Limits continue to live on the
// arena because the inner functions only need read access.

// ── Integers ───────────────────────────────────────────────────────────────

#[inline]
fn parse_small_big<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u8()? as usize;
    if len > arena.limits().max_binary_size {
        return Err(EtfError::BinaryTooLarge);
    }
    let sign = cursor.read_u8()?;
    Ok(Term::SmallBigInt {
        sign,
        digits: cursor.take(len)?,
    })
}

#[inline]
fn parse_large_big<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_binary_size {
        return Err(EtfError::BinaryTooLarge);
    }
    let sign = cursor.read_u8()?;
    Ok(Term::LargeBigInt {
        sign,
        digits: cursor.take(len)?,
    })
}

// ── Floats ─────────────────────────────────────────────────────────────────

#[inline]
fn parse_legacy_float<'a>(
    cursor: &mut Cursor<'a>,
    _arena: &mut Bump<'a>,
    _depth: &mut usize,
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

// ── Atoms (lazy UTF-8 — bytes stored, validated on demand) ─────────────────

#[inline]
fn parse_atom_utf8<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_atom_len {
        return Err(EtfError::AtomTooLarge);
    }
    let bytes = cursor.take(len)?;
    Ok(Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(bytes) }))
}

#[inline]
fn parse_small_atom_utf8<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u8()? as usize;
    if len > arena.limits().max_atom_len {
        return Err(EtfError::AtomTooLarge);
    }
    let bytes = cursor.take(len)?;
    Ok(Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(bytes) }))
}

// ── Tuples ─────────────────────────────────────────────────────────────────

#[inline]
fn parse_small_tuple<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let arity = cursor.read_u8()? as usize;
    parse_tuple_elements(cursor, arena, arity, depth).map(Term::Tuple)
}

#[inline]
fn parse_large_tuple<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let arity = cursor.read_u32()? as usize;
    parse_tuple_elements(cursor, arena, arity, depth).map(Term::Tuple)
}

/// Shared helper: allocate and recursively parse `arity` elements.
#[inline]
fn parse_tuple_elements<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    arity: usize,
    depth: &mut usize,
) -> Result<&'a [Term<'a>], EtfError> {
    if arity > arena.limits().max_tuple_arity {
        return Err(EtfError::MapTooLarge);
    }
    let elements = arena.alloc_slice(arity)?;
    for elem in elements.iter_mut() {
        *elem = parse_term(cursor, arena, depth)?;
    }
    Ok(elements)
}
// ── Lists / Strings ────────────────────────────────────────────────────────

#[inline]
fn parse_string<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_string_len {
        return Err(EtfError::ListTooLarge);
    }
    let bytes = cursor.take(len)?;
    let elements = arena.alloc_slice(len)?;
    for (elem, &b) in elements.iter_mut().zip(bytes.iter()) {
        *elem = Term::Int(b as i32);
    }
    Ok(Term::List(elements))
}

#[inline]
fn parse_list<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_list_len {
        return Err(EtfError::ListTooLarge);
    }
    let elements = arena.alloc_slice(len)?;
    for elem in elements.iter_mut() {
        *elem = parse_term(cursor, arena, depth)?;
    }

    let tail = parse_term(cursor, arena, depth)?;
    match tail {
        Term::List([]) => Ok(Term::List(elements)),
        _ => {
            let tail_ref = arena.alloc_term()?;
            *tail_ref = tail;
            Ok(Term::ImproperList {
                elements,
                tail: tail_ref,
            })
        }
    }
}

// ── Maps ───────────────────────────────────────────────────────────────────

#[inline]
fn parse_map<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_map_len {
        return Err(EtfError::MapTooLarge);
    }
    let pairs = arena.alloc_slice::<(Term<'a>, Term<'a>)>(len)?;
    for pair in pairs.iter_mut() {
        let key = parse_term(cursor, arena, depth)?;
        let value = parse_term(cursor, arena, depth)?;
        *pair = (key, value);
    }
    Ok(Term::Map(pairs))
}

// ── Binaries ───────────────────────────────────────────────────────────────

#[inline]
fn parse_binary<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let len = cursor.read_u32()? as usize;
    if len > arena.limits().max_binary_size {
        return Err(EtfError::BinaryTooLarge);
    }
    Ok(Term::Binary(cursor.take(len)?))
}

#[inline]
fn parse_bit_binary<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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

// ── Process identifiers (PIDs) ─────────────────────────────────────────────

#[inline]
fn parse_pid_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _ = parse_term(cursor, arena, depth)?;
    let _data = cursor.take(9)?;
    let end = cursor.consumed();
    Ok(Term::Pid(Pid(PID_EXT, cursor.slice_between(start, end))))
}

#[inline]
fn parse_pid_new<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _ = parse_term(cursor, arena, depth)?;
    let _data = cursor.take(12)?;
    let end = cursor.consumed();
    Ok(Term::Pid(Pid(
        NEW_PID_EXT,
        cursor.slice_between(start, end),
    )))
}

// ── Ports ──────────────────────────────────────────────────────────────────

#[inline]
fn parse_port_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _ = parse_term(cursor, arena, depth)?;
    let _data = cursor.take(5)?;
    let end = cursor.consumed();
    Ok(Term::Port(Port(PORT_EXT, cursor.slice_between(start, end))))
}

#[inline]
fn parse_port_new<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _ = parse_term(cursor, arena, depth)?;
    let _data = cursor.take(8)?;
    let end = cursor.consumed();
    Ok(Term::Port(Port(
        NEW_PORT_EXT,
        cursor.slice_between(start, end),
    )))
}

#[inline]
fn parse_port_v4<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _ = parse_term(cursor, arena, depth)?;
    let _data = cursor.take(12)?;
    let end = cursor.consumed();
    Ok(Term::Port(Port(
        V4_PORT_EXT,
        cursor.slice_between(start, end),
    )))
}

// ── References ─────────────────────────────────────────────────────────────

#[inline]
fn parse_ref_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_reference_words {
        return Err(EtfError::ListTooLarge);
    }
    let _ = parse_term(cursor, arena, depth)?;
    let _creation = cursor.read_u8()?;
    let _ids = cursor.take(len * 4)?;
    let end = cursor.consumed();
    Ok(Term::Ref(Reference(
        NEW_REFERENCE_EXT,
        cursor.slice_between(start, end),
    )))
}

#[inline]
fn parse_ref_newer<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let len = cursor.read_u16()? as usize;
    if len > arena.limits().max_reference_words {
        return Err(EtfError::ListTooLarge);
    }
    let _ = parse_term(cursor, arena, depth)?;
    let _creation = cursor.read_u32()?;
    let _ids = cursor.take(len * 4)?;
    let end = cursor.consumed();
    Ok(Term::Ref(Reference(
        NEWER_REFERENCE_EXT,
        cursor.slice_between(start, end),
    )))
}

// ── Functions ──────────────────────────────────────────────────────────────

#[inline]
fn parse_new_fun<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    __depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let size = cursor.read_u32()? as usize;
    let remaining = size.checked_sub(4).ok_or(EtfError::InvalidSize)?;
    if remaining > arena.limits().max_fun_size {
        return Err(EtfError::BinaryTooLarge);
    }
    Ok(Term::Function(Function(
        NEW_FUN_EXT,
        cursor.take(remaining)?,
    )))
}

#[inline]
fn parse_export<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _module = parse_term(cursor, arena, depth)?;
    let _function = parse_term(cursor, arena, depth)?;
    let _arity = parse_term(cursor, arena, depth)?;
    let end = cursor.consumed();
    Ok(Term::Function(Function(
        EXPORT_EXT,
        cursor.slice_between(start, end),
    )))
}

// ── Records ────────────────────────────────────────────────────────────────

#[inline]
fn parse_record<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let num_fields = cursor.read_u32()? as usize;
    if num_fields > arena.limits().max_map_len {
        return Err(EtfError::MapTooLarge);
    }
    let _flags = cursor.read_u8()?;
    let _module = parse_term(cursor, arena, depth)?;
    let _name = parse_term(cursor, arena, depth)?;
    for _ in 0..num_fields {
        let _ = parse_term(cursor, arena, depth)?;
    }
    for _ in 0..num_fields {
        let _ = parse_term(cursor, arena, depth)?;
    }
    let end = cursor.consumed();
    Ok(Term::Record(Record(cursor.slice_between(start, end))))
}
