use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::{AtomUtf8, Term};

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
///
/// Branch ordering is based on typical Erlang message frequency:
/// 1. SMALL_INTEGER_EXT (most common integer representation)
/// 2. ATOM_UTF8_EXT / SMALL_ATOM_UTF8_EXT (atoms are very common)
/// 3. NIL_EXT / LIST_EXT (lists are common)
/// 4. SMALL_TUPLE_EXT (tuples are common)
/// 5. INTEGER_EXT, NEW_FLOAT_EXT (other simple types)
/// 6. Rest in alphabetical/complexity order
#[inline(always)]
fn parse_term_inner<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let tag = cursor.read_u8()?;

    // ── Fast path: SMALL_INTEGER_EXT (most frequent tag) ──────────────
    // Checked first because small integers are the most common term type
    // in typical Erlang messages, improving branch prediction.
    if tag == SMALL_INTEGER_EXT {
        // SAFETY: We just read a tag byte, so cursor has at least 1 byte.
        // SMALL_INTEGER_EXT requires exactly 1 more byte for the value.
        if !cursor.data.is_empty() {
            unsafe {
                return Ok(Term::Int(cursor.read_u8_unchecked() as i32));
            }
        }
        return Err(cursor.eof_or_incomplete(1));
    }

    // ── Single consolidated match ordered by frequency ─────────────────
    match tag {
        // Simple, no-allocation types first (fastest to parse)
        INTEGER_EXT => {
            // SAFETY: INTEGER_EXT requires exactly 4 bytes for the value.
            if cursor.data.len() >= 4 {
                unsafe {
                    return Ok(Term::Int(cursor.read_u32_unchecked() as i32));
                }
            }
            Err(cursor.eof_or_incomplete(4))
        }
        NEW_FLOAT_EXT => {
            // SAFETY: NEW_FLOAT_EXT requires exactly 8 bytes for the value.
            if cursor.data.len() >= 8 {
                unsafe {
                    return Ok(Term::Float(cursor.read_f64_unchecked()));
                }
            }
            Err(cursor.eof_or_incomplete(8))
        }
        NIL_EXT => Ok(Term::List(&[])),

        // Atoms (very common in Erlang messages)
        ATOM_UTF8_EXT => parse_atom_utf8(cursor, arena, depth),
        SMALL_ATOM_UTF8_EXT => parse_small_atom_utf8(cursor, arena, depth),

        // Lists and tuples (common compound types)
        SMALL_TUPLE_EXT => parse_small_tuple(cursor, arena, depth),
        LARGE_TUPLE_EXT => parse_large_tuple(cursor, arena, depth),
        LIST_EXT => parse_list(cursor, arena, depth),
        STRING_EXT => parse_string(cursor, arena, depth),

        // Maps (increasingly common)
        MAP_EXT => parse_map(cursor, arena, depth),

        // Binaries
        BINARY_EXT => parse_binary(cursor, arena, depth),
        BIT_BINARY_EXT => parse_bit_binary(cursor, arena, depth),

        // Bignums
        SMALL_BIG_EXT => parse_small_big(cursor, arena, depth),
        LARGE_BIG_EXT => parse_large_big(cursor, arena, depth),

        // Floats (legacy)
        FLOAT_EXT => parse_legacy_float(cursor, arena, depth),

        // Process identifiers
        PID_EXT => parse_pid_legacy(cursor, arena, depth),
        NEW_PID_EXT => parse_pid_new(cursor, arena, depth),

        // Ports
        PORT_EXT => parse_port_legacy(cursor, arena, depth),
        NEW_PORT_EXT => parse_port_new(cursor, arena, depth),
        V4_PORT_EXT => parse_port_v4(cursor, arena, depth),

        // References
        NEW_REFERENCE_EXT => parse_ref_legacy(cursor, arena, depth),
        NEWER_REFERENCE_EXT => parse_ref_newer(cursor, arena, depth),

        // Functions
        NEW_FUN_EXT => parse_new_fun(cursor, arena, depth),
        EXPORT_EXT => parse_export(cursor, arena, depth),

        // Records (OTP 29+)
        RECORD_EXT => parse_record(cursor, arena, depth),

        // Unsupported tags
        LOCAL_EXT | COMPRESSED | ATOM_CACHE_REF => Err(EtfError::UnsupportedTag(tag)),

        // Unknown tag
        _ => Err(EtfError::UnsupportedTag(tag)),
    }
}

// ── Integers ───────────────────────────────────────────────────────────────

#[inline]
fn parse_small_big<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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
fn parse_large_big<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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

/// Parse an atom without recursing into arbitrary terms.
///
/// This is used for fields that MUST be atoms (node, module, function names)
/// to prevent resource exhaustion attacks where a crafted payload substitutes
/// a deeply nested term for an atom field.
#[inline]
fn parse_atom_only<'a>(
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
        return Err(EtfError::TupleTooLarge);
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

    // A1: Opt-in compact STRING_EXT representation
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
    _depth: &mut usize,
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
fn parse_pid_new<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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
fn parse_port_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(5)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Port(slice))
}

#[inline]
fn parse_port_new<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed();
    let _node = parse_atom_only(cursor, arena)?;
    let _data = cursor.take(8)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start - 1, end);
    Ok(Term::Port(slice))
}

#[inline]
fn parse_port_v4<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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
fn parse_ref_legacy<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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
fn parse_ref_newer<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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
fn parse_new_fun<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    _depth: &mut usize,
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
fn parse_export<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed() - 1; // Include tag byte
    let _module = parse_atom_only(cursor, arena)?;
    let _function = parse_atom_only(cursor, arena)?;
    let _arity = parse_term(cursor, arena, depth)?;
    let end = cursor.consumed();
    let slice = cursor.slice_between(start, end);
    Ok(Term::Function(slice))
}

// ── Records ────────────────────────────────────────────────────────────────

#[inline]
fn parse_record<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
    depth: &mut usize,
) -> Result<Term<'a>, EtfError> {
    let start = cursor.consumed() - 1; // Include tag byte
    let num_fields = cursor.read_u32()? as usize;
    if num_fields > arena.limits().max_map_len {
        return Err(EtfError::MapTooLarge);
    }
    let _flags = cursor.read_u8()?;
    let _module = parse_atom_only(cursor, arena)?;
    let _name = parse_atom_only(cursor, arena)?;
    for _ in 0..num_fields {
        let _ = parse_atom_only(cursor, arena)?;
    }
    for _ in 0..num_fields {
        let _ = parse_term(cursor, arena, depth)?;
    }
    let end = cursor.consumed();
    let slice = cursor.slice_between(start, end);
    Ok(Term::Record(slice))
}
