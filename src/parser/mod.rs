use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::Term;

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
    depth: usize,
) -> Result<Term<'a>, EtfError> {
    if depth > arena.limits().max_depth {
        return Err(EtfError::RecursionLimitExceeded);
    }
    parse_term_inner(cursor, arena, depth)
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
    depth: usize,
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

mod atom;
mod compound;
mod opaque;
mod scalar;

use self::atom::*;
use self::compound::*;
use self::opaque::*;
use self::scalar::*;
