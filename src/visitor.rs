// ─────────────────────────────────────────────────────────────────────────────
// Visitor API
//
// Instead of constructing a full AST, callers can implement the [`Visitor`]
// trait to process terms as they are decoded.  This gives:
//
//   * Zero allocations — no arena required.
//   * Streaming-friendly — events are emitted as the wire is consumed.
//   * Lower peak memory — compound terms are visited element-by-element.
//   * Better embedded support — works without an arena buffer.
//
// The visitor events mirror the structure of the [`Term`](crate::Term) enum
// but flattened into method calls.
//
// # Basic usage
//
// ```ignore
// use fasteetf::{Visitor, parse_etf_with_visitor};
//
// /// A visitor that counts the number of atoms in a term.
// struct AtomCounter(usize);
//
// impl Visitor for AtomCounter {
//     fn visit_atom(&mut self, _bytes: &[u8]) -> Result<(), ()> {
//         self.0 += 1;
//         Ok(())
//     }
// }
//
// let input = b"\x83\x61\x01"; // 131, SMALL_INTEGER_EXT, 1
// let mut visitor = AtomCounter(0);
// parse_etf_with_visitor(input, None, &mut visitor, &Limits::default()).unwrap();
// ```
//
// See the documentation on each method for the corresponding ETF tag.
// ─────────────────────────────────────────────────────────────────────────────

use crate::Limits;
use crate::cursor::Cursor;
use crate::error::EtfError;
use crate::tags::*;
#[cfg(feature = "compression")]
use crate::zlib;

/// A visitor that receives events as an ETF term tree is decoded.
///
/// Implement this trait when you want to process terms without constructing
/// an AST (zero-allocation parsing).
///
/// # Default implementations
///
/// Every method has a default that returns [`Ok`] — override only the events
/// you care about.
///
/// # Error type
///
/// The associated [`Error`](Self::Error) type lets visitors inject their own
/// validation or application-level errors into the parse.  Use `()`,
/// [`EtfError`], or a custom error type.
pub trait Visitor {
    /// The error type returned by visitor methods.
    /// Use `()` or [`EtfError`] for most cases.
    type Error: From<EtfError>;

    // ── Scalars ────────────────────────────────────────────────────────

    /// Called for a small integer or integer term.
    ///
    /// Tags: `SMALL_INTEGER_EXT` (97), `INTEGER_EXT` (98).
    fn visit_int(&mut self, _value: i32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a big integer.
    ///
    /// Tags: `SMALL_BIG_EXT` (110), `LARGE_BIG_EXT` (111).
    fn visit_big_int(&mut self, _sign: u8, _digits: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a float term.
    ///
    /// Tags: `NEW_FLOAT_EXT` (70), `FLOAT_EXT` (99).
    fn visit_float(&mut self, _value: f64) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for an atom term.
    ///
    /// The bytes are **not** guaranteed to be valid UTF-8 — lazily validate
    /// with `core::str::from_utf8` if you need a `&str`.
    ///
    /// Tags: `ATOM_UTF8_EXT` (118), `SMALL_ATOM_UTF8_EXT` (119).
    fn visit_atom(&mut self, _bytes: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    // ── Binaries ───────────────────────────────────────────────────────

    /// Called for a binary term.
    ///
    /// Tag: `BINARY_EXT` (109).
    fn visit_binary(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a bit-binary term.
    ///
    /// Tag: `BIT_BINARY_EXT` (77).
    fn visit_bit_binary(&mut self, _bits: u8, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    // ── Compound terms ─────────────────────────────────────────────────

    /// Called at the start of a tuple.
    ///
    /// `arity` is the number of elements.
    ///
    /// Tags: `SMALL_TUPLE_EXT` (104), `LARGE_TUPLE_EXT` (105).
    fn visit_tuple_start(&mut self, _arity: usize) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the end of a tuple.
    fn visit_tuple_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the start of a list.
    ///
    /// `len` is the number of elements before the tail.
    ///
    /// Tags: `LIST_EXT` (108).
    fn visit_list_start(&mut self, _len: usize) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the end of a proper list (after all elements and a nil
    /// tail have been consumed).
    fn visit_list_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the start of the tail of an improper list.
    ///
    /// After this event, exactly one term will be visited for the tail,
    /// then [`visit_improper_list_end`](Self::visit_improper_list_end)
    /// will be called.
    fn visit_improper_list_tail(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the end of an improper list.
    fn visit_improper_list_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the start of a map.
    ///
    /// `arity` is the number of key-value pairs.
    ///
    /// Tag: `MAP_EXT` (116).
    fn visit_map_start(&mut self, _arity: usize) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called at the end of a map.
    fn visit_map_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    // ── Opaque identifier wrappers ─────────────────────────────────────

    /// Called for a PID term.
    ///
    /// `data` contains the raw bytes after the node atom.
    ///
    /// Tags: `PID_EXT` (103), `NEW_PID_EXT` (88).
    fn visit_pid(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a port term.
    ///
    /// Tags: `PORT_EXT` (102), `NEW_PORT_EXT` (89), `V4_PORT_EXT` (120).
    fn visit_port(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a reference term.
    ///
    /// Tags: `NEW_REFERENCE_EXT` (114), `NEWER_REFERENCE_EXT` (90).
    fn visit_reference(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a function term.
    ///
    /// Tags: `NEW_FUN_EXT` (112), `EXPORT_EXT` (113).
    fn visit_function(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called for a record term.
    ///
    /// Tag: `RECORD_EXT` (67).
    fn visit_record(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    // ── String (list-of-bytes optimisation) ────────────────────────────

    /// Called for a string optimisation.
    ///
    /// This is an optimisation for lists of bytes (integers 0–255).  The
    /// visitor receives the raw bytes directly rather than individual
    /// integer terms.
    ///
    /// Tag: `STRING_EXT` (107).
    fn visit_string(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}

// ── Visitor-based parser ────────────────────────────────────────────────────

/// Parse an ETF-encoded term using a [`Visitor`] instead of building an AST.
///
/// This is the zero-allocation entry-point: no arena is needed, and compound
/// terms are streamed as nested enter/leave events.
///
/// The compression wrapper (`131 80 …`) is handled transparently when the
/// `compression` feature is enabled and a decompression buffer is provided.
///
/// Resource limits are controlled by `limits`.
pub fn parse_etf_with_visitor<'a, V: Visitor>(
    input: &'a [u8],
    #[allow(unused_variables)] decompressed_buffer: Option<&'a mut [u8]>,
    #[allow(unused_variables)] zlib_backend: Option<crate::ZlibDecompressFn>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let mut cursor = Cursor::new(input);

    // Magic byte.
    let magic = cursor.take(1)?[0];
    if magic != 131 {
        return Err(EtfError::InvalidMagicNumber.into());
    }

    // Compression wrapper.
    #[cfg(feature = "compression")]
    if cursor.data.first() == Some(&COMPRESSED) {
        cursor.take(1)?;
        let uncompressed_size = cursor.read_u32()? as usize;

        let decomp_buf = decompressed_buffer.ok_or(EtfError::InsufficientDecompressionBuffer)?;
        if decomp_buf.len() < uncompressed_size {
            return Err(EtfError::InsufficientDecompressionBuffer.into());
        }

        let target_buf = &mut decomp_buf[..uncompressed_size];
        zlib::decompress(target_buf, cursor.data, zlib_backend)?;

        let mut dec_cursor = Cursor::new(target_buf);
        return visit_term(&mut dec_cursor, visitor, 0, limits);
    }

    #[cfg(not(feature = "compression"))]
    {
        let _ = zlib_backend;
        if cursor.data.first() == Some(&COMPRESSED) {
            return Err(EtfError::UnsupportedTag(COMPRESSED).into());
        }
    }

    visit_term(&mut cursor, visitor, 0, limits)
}

/// Parse an ETF-encoded term using a [`Visitor`] from a **potentially
/// incomplete** input buffer.
///
/// This is the streaming / incremental zero-allocation entry point.
/// When the input does not contain a complete term, this function returns
/// [`EtfError::Incomplete(Needed)`] with the minimum number of additional
/// bytes required.
///
/// See [`parse_etf_streaming`](crate::parse_etf_streaming) for the
/// AST-based equivalent and a usage example.
///
/// Resource limits are controlled by `limits`.
pub fn parse_etf_with_visitor_streaming<'a, V: Visitor>(
    input: &'a [u8],
    #[allow(unused_variables)] decompressed_buffer: Option<&'a mut [u8]>,
    #[allow(unused_variables)] zlib_backend: Option<crate::ZlibDecompressFn>,
    visitor: &mut V,
    limits: &Limits,
) -> Result<(), V::Error> {
    let mut cursor = Cursor::new_streaming(input);

    // Magic byte.
    let magic = cursor.take(1)?[0];
    if magic != 131 {
        return Err(EtfError::InvalidMagicNumber.into());
    }

    // Compression wrapper.
    #[cfg(feature = "compression")]
    if cursor.data.first() == Some(&COMPRESSED) {
        cursor.take(1)?;
        let uncompressed_size = cursor.read_u32()? as usize;

        let decomp_buf = decompressed_buffer.ok_or(EtfError::InsufficientDecompressionBuffer)?;
        if decomp_buf.len() < uncompressed_size {
            return Err(EtfError::InsufficientDecompressionBuffer.into());
        }

        let target_buf = &mut decomp_buf[..uncompressed_size];
        zlib::decompress(target_buf, cursor.data, zlib_backend)?;

        let mut dec_cursor = Cursor::new(target_buf);
        return visit_term(&mut dec_cursor, visitor, 0, limits);
    }

    #[cfg(not(feature = "compression"))]
    {
        let _ = zlib_backend;
        if cursor.data.first() == Some(&COMPRESSED) {
            return Err(EtfError::UnsupportedTag(COMPRESSED).into());
        }
    }

    visit_term(&mut cursor, visitor, 0, limits)
}

/// Internal recursive dispatch for the visitor-based parser.
///
/// Delegates to small specialized functions — one per tag or tag family —
/// so that each visit path can be fuzzed, audited, and maintained
/// independently.
fn visit_term<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    if depth > limits.max_depth {
        return Err(EtfError::RecursionLimitExceeded.into());
    }

    let tag = cursor.read_u8()?;

    // Fast path: small integer.
    if tag == SMALL_INTEGER_EXT {
        return visitor.visit_int(cursor.read_u8()? as i32);
    }

    // SMALL_INTEGER_EXT is handled by the fast path before the match.
    // It is intentionally omitted here — if it somehow reaches the match
    // it will be caught by the catch-all instead of panicking.

    match tag {
        INTEGER_EXT => visit_int(cursor, visitor),

        SMALL_BIG_EXT => visit_small_big(cursor, visitor, limits),
        LARGE_BIG_EXT => visit_large_big(cursor, visitor, limits),

        NEW_FLOAT_EXT => visit_new_float(cursor, visitor),
        FLOAT_EXT => visit_legacy_float(cursor, visitor),

        ATOM_UTF8_EXT => visit_atom_utf8(cursor, visitor, limits),
        SMALL_ATOM_UTF8_EXT => visit_small_atom_utf8(cursor, visitor, limits),

        SMALL_TUPLE_EXT => visit_small_tuple(cursor, visitor, depth, limits),
        LARGE_TUPLE_EXT => visit_large_tuple(cursor, visitor, depth, limits),

        NIL_EXT => visit_nil(visitor),
        STRING_EXT => visit_string(cursor, visitor, limits),
        LIST_EXT => visit_list(cursor, visitor, depth, limits),

        MAP_EXT => visit_map(cursor, visitor, depth, limits),

        BINARY_EXT => visit_binary(cursor, visitor, limits),
        BIT_BINARY_EXT => visit_bit_binary(cursor, visitor, limits),

        PID_EXT => visit_pid_legacy(cursor, visitor, depth, limits),
        NEW_PID_EXT => visit_pid_new(cursor, visitor, depth, limits),

        PORT_EXT => visit_port_legacy(cursor, visitor, depth, limits),
        NEW_PORT_EXT => visit_port_new(cursor, visitor, depth, limits),
        V4_PORT_EXT => visit_port_v4(cursor, visitor, depth, limits),

        NEW_REFERENCE_EXT => visit_ref_legacy(cursor, visitor, depth, limits),
        NEWER_REFERENCE_EXT => visit_ref_newer(cursor, visitor, depth, limits),

        NEW_FUN_EXT => visit_new_fun(cursor, visitor, limits),
        EXPORT_EXT => visit_export(cursor, visitor, depth, limits),

        RECORD_EXT => visit_record(cursor, visitor, depth, limits),

        _ => Err(EtfError::UnsupportedTag(tag).into()),
    }
}

// ── Integers ───────────────────────────────────────────────────────────────

fn visit_int<'a, V: Visitor>(cursor: &mut Cursor<'a>, visitor: &mut V) -> Result<(), V::Error> {
    visitor.visit_int(cursor.read_u32()? as i32)
}

// ── Bignums ────────────────────────────────────────────────────────────────

fn visit_small_big<'a, V: Visitor>(
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

fn visit_large_big<'a, V: Visitor>(
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

fn visit_new_float<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
) -> Result<(), V::Error> {
    visitor.visit_float(cursor.read_f64()?)
}

fn visit_legacy_float<'a, V: Visitor>(
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

// ── Atoms ──────────────────────────────────────────────────────────────────

fn visit_atom_utf8<'a, V: Visitor>(
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

fn visit_small_atom_utf8<'a, V: Visitor>(
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

// ── Tuples ─────────────────────────────────────────────────────────────────

fn visit_small_tuple<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let arity = cursor.read_u8()? as usize;
    if arity > limits.max_tuple_arity {
        return Err(EtfError::MapTooLarge.into());
    }
    visitor.visit_tuple_start(arity)?;
    for _ in 0..arity {
        visit_term(cursor, visitor, depth + 1, limits)?;
    }
    visitor.visit_tuple_end()
}

fn visit_large_tuple<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    let arity = cursor.read_u32()? as usize;
    if arity > limits.max_tuple_arity {
        return Err(EtfError::MapTooLarge.into());
    }
    visitor.visit_tuple_start(arity)?;
    for _ in 0..arity {
        visit_term(cursor, visitor, depth + 1, limits)?;
    }
    visitor.visit_tuple_end()
}

// ── Lists / Nil / Strings ──────────────────────────────────────────────────

fn visit_nil<V: Visitor>(visitor: &mut V) -> Result<(), V::Error> {
    visitor.visit_list_start(0)?;
    visitor.visit_list_end()
}

fn visit_string<'a, V: Visitor>(
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

fn visit_list<'a, V: Visitor>(
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

fn visit_map<'a, V: Visitor>(
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

// ── Binaries ───────────────────────────────────────────────────────────────

fn visit_binary<'a, V: Visitor>(
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

fn visit_bit_binary<'a, V: Visitor>(
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

// ── PIDs ───────────────────────────────────────────────────────────────────

fn visit_pid_legacy<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?; // node atom
    let data = cursor.take(9)?;
    visitor.visit_pid(data)
}

fn visit_pid_new<'a, V: Visitor>(
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

fn visit_port_legacy<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?;
    let data = cursor.take(5)?;
    visitor.visit_port(data)
}

fn visit_port_new<'a, V: Visitor>(
    cursor: &mut Cursor<'a>,
    visitor: &mut V,
    depth: usize,
    limits: &Limits,
) -> Result<(), V::Error> {
    visit_term(cursor, visitor, depth + 1, limits)?;
    let data = cursor.take(8)?;
    visitor.visit_port(data)
}

fn visit_port_v4<'a, V: Visitor>(
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

fn visit_ref_legacy<'a, V: Visitor>(
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

fn visit_ref_newer<'a, V: Visitor>(
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

fn visit_new_fun<'a, V: Visitor>(
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

fn visit_export<'a, V: Visitor>(
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

fn visit_record<'a, V: Visitor>(
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
