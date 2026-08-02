use crate::arena::Bump;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::tags::*;
use crate::types::Term;

// ── Iterative parse frames ──────────────────────────────────────────────────
//
// Parsing is fully iterative: instead of recursive `parse_term` calls, one
// *frame* per in-flight compound term is pushed onto an explicit stack.
// Frames are small bump allocations from the arena (one per *nesting level*,
// not per element), so deeply nested input never grows the process stack —
// `max_depth` remains the only bound on memory, exactly as before.
//
// A frame describes what to do with the next completed child term:
//
// * `Tuple` — write it into `base[next]`; when all `len` slots are filled,
//   emit `Term::Tuple`.
// * `List` — write into `base[next]`; slots `base[0..len]` hold the
//   elements and `base[len]` the tail.  The tail slot decides between
//   `Term::List` (proper) and `Term::ImproperList`.
// * `Map` — write into `base[next/2].{0,1}`; when all `len` pairs are
//   filled, emit `Term::Map`.
// * `Export` / `Record` — the child values are discarded (only the consumed
//   byte range matters); when the child count is exhausted, emit
//   `Term::Function` / `Term::Record` over the consumed slice.

struct Frame<'a> {
    /// The frame below this one on the stack (linked list).
    prev: *mut Frame<'a>,
    kind: FrameKind<'a>,
}

enum FrameKind<'a> {
    Tuple {
        base: *mut Term<'a>,
        len: u32,
        next: u32,
    },
    List {
        base: *mut Term<'a>,
        len: u32,
        next: u32,
    },
    Map {
        base: *mut (Term<'a>, Term<'a>),
        len: u32,
        next: u32,
    },
    Export {
        /// Offset of the tag byte in the original input.
        start: usize,
        /// Throwaway slot receiving the arity term.
        slot: *mut Term<'a>,
    },
    Record {
        start: usize,
        /// Remaining child terms to parse (module, name, fields).
        remaining: u64,
        /// Throwaway slot receiving each child term.
        slot: *mut Term<'a>,
    },
}

/// Write a completed term into the waiting slot of the top frame, advancing
/// or finalizing frames until a frame still has children to fill.
///
/// Returns `Some(term)` when the root term is complete.
#[inline(always)]
fn complete<'a>(
    mut value: Term<'a>,
    cursor: &mut Cursor<'a>,
    top: &mut *mut Frame<'a>,
    height: &mut usize,
) -> Option<Term<'a>> {
    loop {
        let frame_ptr = *top;
        if frame_ptr.is_null() {
            return Some(value);
        }
        // SAFETY: `frame_ptr` is a live arena-allocated frame; it stays
        // alive until popped and is never aliased while on the stack.
        let frame = unsafe { &mut *frame_ptr };
        match &mut frame.kind {
            FrameKind::Tuple { base, len, next } => {
                // SAFETY: `*next < *len` — the frame is only on the stack
                // while children remain, and it finalizes once `next` hits
                // `len` (zero-arity tuples complete without a frame).
                unsafe { base.add(*next as usize).write(value) };
                *next += 1;
                if *next < *len {
                    return None;
                }
                value = Term::Tuple(unsafe { core::slice::from_raw_parts(*base, *len as usize) });
            }
            FrameKind::List { base, len, next } => {
                // SAFETY: `*next <= *len` — slots `base[0..len]` hold the
                // elements and `base[len]` the tail; finalize at `len + 1`.
                unsafe { base.add(*next as usize).write(value) };
                *next += 1;
                if *next <= *len {
                    return None;
                }
                let tail = unsafe { *base.add(*len as usize) };
                let len_usize = *len as usize;
                value = if matches!(tail, Term::List([])) {
                    Term::List(unsafe { core::slice::from_raw_parts(*base, len_usize) })
                } else {
                    Term::ImproperList(unsafe { core::slice::from_raw_parts(*base, len_usize + 1) })
                };
            }
            FrameKind::Map { base, len, next } => {
                // SAFETY: `*next < 2 * *len` while children remain
                // (`*next >> 1` indexes the pair, `*next & 1` the field).
                let pair_ptr = unsafe { base.add((*next >> 1) as usize) };
                let slot = if *next & 1 == 0 {
                    unsafe { core::ptr::addr_of_mut!((*pair_ptr).0) }
                } else {
                    unsafe { core::ptr::addr_of_mut!((*pair_ptr).1) }
                };
                unsafe { slot.write(value) };
                *next += 1;
                if (*next >> 1) < *len {
                    return None;
                }
                value = Term::Map(unsafe { core::slice::from_raw_parts(*base, *len as usize) });
            }
            FrameKind::Export { start, slot } => {
                // SAFETY: `slot` is a live 1-element arena allocation.
                unsafe { (*slot).write(value) };
                let end = cursor.consumed();
                value = Term::Function(cursor.slice_between(*start, end));
            }
            FrameKind::Record {
                start,
                remaining,
                slot,
            } => {
                // SAFETY: `slot` is a live 1-element arena allocation.
                unsafe { (*slot).write(value) };
                *remaining -= 1;
                if *remaining > 0 {
                    return None;
                }
                let end = cursor.consumed();
                value = Term::Record(cursor.slice_between(*start, end));
            }
        }
        // The frame finalized: pop it and propagate the value upward.
        let prev = frame.prev;
        *top = prev;
        *height -= 1;
    }
}

/// Push a new in-flight compound frame onto the stack.
#[inline(always)]
fn push_frame<'a>(
    top: &mut *mut Frame<'a>,
    height: &mut usize,
    arena: &mut Bump<'a>,
    kind: FrameKind<'a>,
) -> Result<(), EtfError> {
    let slots = arena.alloc_slice::<Frame>(1)?;
    slots[0] = Frame { prev: *top, kind };
    *top = slots.as_mut_ptr();
    *height += 1;
    Ok(())
}

/// Complete a term: write it into the frame stack, continuing if a frame
/// still has children to fill, or return the root value.
macro_rules! complete_or_continue {
    ($value:expr, $cursor:expr, $top:expr, $height:expr) => {
        if let Some(v) = complete($value, $cursor, $top, $height) {
            return Ok(v);
        }
    };
}

/// Parse a single ETF term from `cursor`, allocating compound storage from
/// `arena`.
///
/// ## Fast path
///
/// The small-integer tag (`97`) is checked first because small integer terms
/// are the most frequently encountered term type in typical Erlang messages.
/// This single comparison saves a full match against every tag variant.
///
/// ## Iterative traversal
///
/// Compound terms push a [`Frame`] onto an explicit stack instead of
/// recursing (see the module docs above); `height` tracks the current
/// nesting depth and is checked against `Limits::max_depth` exactly like
/// the old recursion budget.
#[inline(always)]
pub(crate) fn parse_term<'a>(
    cursor: &mut Cursor<'a>,
    arena: &mut Bump<'a>,
) -> Result<Term<'a>, EtfError> {
    let mut top: *mut Frame<'a> = core::ptr::null_mut();
    let mut height = 0usize;

    loop {
        // Nesting budget: `height` is the number of in-flight compound
        // frames, i.e. the current nesting depth.
        if height > arena.limits().max_depth {
            return Err(EtfError::RecursionLimitExceeded);
        }

        let tag = cursor.read_u8()?;

        // ── Fast path: SMALL_INTEGER_EXT (most frequent tag) ──────────────
        // Checked first because small integers are the most common term type
        // in typical Erlang messages, improving branch prediction.
        if tag == SMALL_INTEGER_EXT {
            // SAFETY: We just read a tag byte, so cursor has at least 1 byte.
            // SMALL_INTEGER_EXT requires exactly 1 more byte for the value.
            if cursor.data.is_empty() {
                return Err(cursor.eof_or_incomplete(1));
            }
            let value = Term::Int(unsafe { cursor.read_u8_unchecked() } as i32);
            complete_or_continue!(value, cursor, &mut top, &mut height);
            continue;
        }

        // ── Single consolidated match ordered by frequency ─────────────────
        match tag {
            // Simple, no-allocation types first (fastest to parse)
            INTEGER_EXT => {
                // SAFETY: INTEGER_EXT requires exactly 4 bytes for the value.
                if cursor.data.len() >= 4 {
                    let value = Term::Int(unsafe { cursor.read_u32_unchecked() } as i32);
                    complete_or_continue!(value, cursor, &mut top, &mut height);
                    continue;
                }
                return Err(cursor.eof_or_incomplete(4));
            }
            NEW_FLOAT_EXT => {
                // SAFETY: NEW_FLOAT_EXT requires exactly 8 bytes for the value.
                if cursor.data.len() >= 8 {
                    let value = Term::Float(unsafe { cursor.read_f64_unchecked() });
                    complete_or_continue!(value, cursor, &mut top, &mut height);
                    continue;
                }
                return Err(cursor.eof_or_incomplete(8));
            }
            NIL_EXT => {
                complete_or_continue!(Term::List(&[]), cursor, &mut top, &mut height);
                continue;
            }

            // Atoms (very common in Erlang messages)
            ATOM_UTF8_EXT => {
                let value = parse_atom_utf8(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            SMALL_ATOM_UTF8_EXT => {
                let value = parse_small_atom_utf8(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Tuples
            SMALL_TUPLE_EXT | LARGE_TUPLE_EXT => {
                let arity = if tag == SMALL_TUPLE_EXT {
                    cursor.read_u8()? as u32
                } else {
                    cursor.read_u32()?
                };
                if arity as usize > arena.limits().max_tuple_arity {
                    return Err(EtfError::TupleTooLarge);
                }
                let elements = arena.alloc_slice::<Term>(arity as usize)?;
                if arity == 0 {
                    complete_or_continue!(Term::Tuple(elements), cursor, &mut top, &mut height);
                    continue;
                }
                push_frame(
                    &mut top,
                    &mut height,
                    arena,
                    FrameKind::Tuple {
                        base: elements.as_mut_ptr(),
                        len: arity,
                        next: 0,
                    },
                )?;
                continue;
            }

            // Lists
            LIST_EXT => {
                let len = cursor.read_u32()?;
                if len as usize > arena.limits().max_list_len {
                    return Err(EtfError::ListTooLarge);
                }
                // `len + 1` slots: the last one holds the tail, so improper
                // lists never need a second allocation or an element copy.
                let total = (len as usize)
                    .checked_add(1)
                    .ok_or(EtfError::ArenaExhausted)?;
                let elements = arena.alloc_slice::<Term>(total)?;
                push_frame(
                    &mut top,
                    &mut height,
                    arena,
                    FrameKind::List {
                        base: elements.as_mut_ptr(),
                        len,
                        next: 0,
                    },
                )?;
                continue;
            }
            STRING_EXT => {
                let value = parse_string(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Maps (increasingly common)
            MAP_EXT => {
                let len = cursor.read_u32()?;
                if len as usize > arena.limits().max_map_len {
                    return Err(EtfError::MapTooLarge);
                }
                let pairs = arena.alloc_slice::<(Term<'a>, Term<'a>)>(len as usize)?;
                if len == 0 {
                    complete_or_continue!(Term::Map(pairs), cursor, &mut top, &mut height);
                    continue;
                }
                push_frame(
                    &mut top,
                    &mut height,
                    arena,
                    FrameKind::Map {
                        base: pairs.as_mut_ptr(),
                        len,
                        next: 0,
                    },
                )?;
                continue;
            }

            // Binaries
            BINARY_EXT => {
                let value = parse_binary(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            BIT_BINARY_EXT => {
                let value = parse_bit_binary(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Bignums
            SMALL_BIG_EXT => {
                let value = parse_small_big(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            LARGE_BIG_EXT => {
                let value = parse_large_big(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Floats (legacy)
            FLOAT_EXT => {
                let value = parse_legacy_float(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Process identifiers
            PID_EXT => {
                let value = parse_pid_legacy(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            NEW_PID_EXT => {
                let value = parse_pid_new(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Ports
            PORT_EXT => {
                let value = parse_port_legacy(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            NEW_PORT_EXT => {
                let value = parse_port_new(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            V4_PORT_EXT => {
                let value = parse_port_v4(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // References
            NEW_REFERENCE_EXT => {
                let value = parse_ref_legacy(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            NEWER_REFERENCE_EXT => {
                let value = parse_ref_newer(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }

            // Functions
            NEW_FUN_EXT => {
                let value = parse_new_fun(cursor, arena)?;
                complete_or_continue!(value, cursor, &mut top, &mut height);
                continue;
            }
            EXPORT_EXT => {
                let start = cursor.consumed() - 1; // include the tag byte
                let _module = parse_atom_only(cursor, arena)?;
                let _function = parse_atom_only(cursor, arena)?;
                let slot = arena.alloc_slice::<Term>(1)?;
                push_frame(
                    &mut top,
                    &mut height,
                    arena,
                    FrameKind::Export {
                        start,
                        slot: slot.as_mut_ptr(),
                    },
                )?;
                continue;
            }

            // Records (OTP 29+)
            RECORD_EXT => {
                let start = cursor.consumed() - 1; // include the tag byte
                let num_fields = cursor.read_u32()?;
                if num_fields as usize > arena.limits().max_map_len {
                    return Err(EtfError::MapTooLarge);
                }
                let _flags = cursor.read_u8()?;
                let slot = arena.alloc_slice::<Term>(1)?;
                // Children: module, name, field names (n), field values (n).
                let remaining = match (num_fields as u64)
                    .checked_mul(2)
                    .and_then(|v| v.checked_add(2))
                {
                    Some(v) => v,
                    None => return Err(EtfError::MapTooLarge),
                };
                push_frame(
                    &mut top,
                    &mut height,
                    arena,
                    FrameKind::Record {
                        start,
                        remaining,
                        slot: slot.as_mut_ptr(),
                    },
                )?;
                continue;
            }

            // Unsupported tags
            LOCAL_EXT | COMPRESSED | ATOM_CACHE_REF => return Err(EtfError::UnsupportedTag(tag)),

            // Unknown tag
            _ => return Err(EtfError::UnsupportedTag(tag)),
        }
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
