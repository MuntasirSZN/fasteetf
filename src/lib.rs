#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
// #![warn(missing_docs)] — uncomment once API is stable

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// ── Internal modules ────────────────────────────────────────────────────────

mod arena;
mod cursor;
mod encoder;
mod error;
mod limits;
mod parser;
#[cfg(feature = "serde")]
mod serde_impl;
mod tags;
mod types;
mod visitor;

// ── Public API surface ──────────────────────────────────────────────────────

pub use encoder::encode_to_buf;

#[cfg(feature = "alloc")]
pub use encoder::encode_to_vec;
pub use error::{EtfError, Needed};
pub use limits::*;
pub use types::{AtomUtf8, Function, Pid, Port, Record, Reference, Term};
pub use visitor::{Visitor, parse_etf_with_visitor, parse_etf_with_visitor_streaming};

#[cfg(feature = "alloc")]
pub use types::owned::{
    self, FunctionOwned, OwnedTerm, PidOwned, PortOwned, RecordOwned, ReferenceOwned,
};

use core::mem::MaybeUninit;

// ── Constants ───────────────────────────────────────────────────────────────

/// Magic version byte.  Every valid ETF stream starts with `131`.
///
/// Spec: https://www.erlang.org/doc/apps/erts/erl_ext_dist#introduction
pub(crate) const ETF_MAGIC: u8 = 131;

// ── Public API ──────────────────────────────────────────────────────────────

/// Options passed to [`parse_etf`].
pub struct ParseOptions<'a> {
    /// The raw ETF byte slice to parse.
    pub input: &'a [u8],
    /// An optional buffer used for decompression.  Must be large enough to
    /// hold the uncompressed data when the input is a compressed ETF stream.
    pub decompressed_buffer: Option<&'a mut [u8]>,
    /// Scratch space used by the bump arena to build the AST.  The required
    /// size depends on the complexity of the term; 8–16 kiB is a good starting
    /// point for most real-world messages.
    pub ast_arena: &'a mut [MaybeUninit<u8>],
    /// Resource limits enforced during parsing.
    ///
    /// Use [`Limits::default()`] for the built-in defaults, or construct a
    /// custom `Limits` with overridden fields for tighter/looser bounds.
    pub limits: Limits,
}

/// Parse an ETF-encoded term from a complete input buffer.
///
/// The wire format is:
///
/// ```text
/// 131 Tag Data…
/// ```
///
/// where `131` is the magic version byte ([`ETF_MAGIC`]) and `Tag` identifies
/// the term type.  See the module-level documentation on each tag for the
/// full format reference.
///
/// ## Compression
///
/// When the input starts with `131 80` (the [`COMPRESSED`] tag) followed by
/// a 4-byte big-endian uncompressed size and a zlib-compressed payload, this
/// function transparently decompresses using the caller-supplied
/// [`ParseOptions::decompressed_buffer`].
///
/// ## Zero-copy
///
/// The returned [`Term`] borrows from either the original `input` slice or
/// the `decompressed_buffer` — no heap allocation occurs.
///
/// ## Errors
///
/// Returns [`EtfError`] on malformed input, unsupported tags, arena
/// exhaustion, or decompression failure.
///
/// Spec: https://www.erlang.org/doc/apps/erts/erl_ext_dist
pub fn parse_etf<'a>(options: ParseOptions<'a>) -> Result<Term<'a>, EtfError> {
    #[cfg(not(feature = "compression"))]
    let _ = options.decompressed_buffer; // suppress unused warning

    let mut cursor = cursor::Cursor::new(options.input);

    // ── Magic byte ─────────────────────────────────────────────────────
    let magic = cursor.take(1)?[0];
    if magic != ETF_MAGIC {
        return Err(EtfError::InvalidMagicNumber);
    }

    // ── Compression wrapper ────────────────────────────────────────────
    // Peek at the next byte without advancing.  If it is the COMPRESSED
    // (80) tag we transparently decompress before recursing.
    if cursor.data.first() == Some(&tags::COMPRESSED) {
        #[cfg(not(feature = "compression"))]
        {
            let _ = cursor; // suppress unused
            return Err(EtfError::UnsupportedTag(tags::COMPRESSED));
        }

        #[cfg(feature = "compression")]
        {
            cursor.take(1)?; // consume tag 80
            let uncompressed_size = cursor.read_u32()? as usize;

            let decomp_buf = options
                .decompressed_buffer
                .ok_or(EtfError::InsufficientDecompressionBuffer)?;

            if decomp_buf.len() < uncompressed_size {
                return Err(EtfError::InsufficientDecompressionBuffer);
            }

            let target_buf = &mut decomp_buf[..uncompressed_size];

            // One-shot zlib decompression via zlib-rs.
            let (_, rc) = zlib_rs::decompress_slice(target_buf, cursor.data, Default::default());
            if rc != zlib_rs::ReturnCode::Ok {
                return Err(EtfError::DecompressionFailed);
            }

            let mut dec_cursor = cursor::Cursor::new(target_buf);
            let mut arena =
                arena::Bump::new(options.ast_arena, options.limits.max_depth + 1, &options.limits);

            parser::parse_term(&mut dec_cursor, &mut arena)
        }
    } else {
        let mut arena =
            arena::Bump::new(options.ast_arena, options.limits.max_depth + 1, &options.limits);

        parser::parse_term(&mut cursor, &mut arena)
    }
}

/// Parse an ETF-encoded term from a **potentially incomplete** input buffer.
///
/// This is the incremental / streaming entry point.  When the input does not
/// contain a complete term, this function returns
/// [`EtfError::Incomplete(Needed)`] with the minimum number of additional
/// bytes required.
///
/// ## Usage pattern
///
/// ```ignore
/// let mut buf = Vec::new();
/// let mut arena = vec![core::mem::MaybeUninit::<u8>::uninit(); 65536];
///
/// loop {
///     // Read more data from socket / file / etc.
///     let n = read_more(&mut buf);
///     if n == 0 {
///         // No more data available — treat as EOF.
///         match parse_etf_streaming(ParseOptions { input: &buf, .. }) {
///             Err(EtfError::UnexpectedEof) => /* incomplete, bail */,
///             result => break result,
///         }
///     }
///
///     match parse_etf_streaming(ParseOptions { input: &buf, .. }) {
///         Ok(term) => break Ok(term),
///         Err(EtfError::Incomplete(needed)) => {
///             // Need more data — ensure buffer has room.
///             buf.reserve(needed.size().unwrap_or(4096));
///             continue;
///         }
///         Err(e) => break Err(e),
///     }
/// }
/// ```
///
/// If the full input is already available, prefer [`parse_etf`] — it returns
/// [`UnexpectedEof`] for truly truncated data, which is easier to distinguish
/// from a mere "need more data" signal.
pub fn parse_etf_streaming<'a>(options: ParseOptions<'a>) -> Result<Term<'a>, EtfError> {
    #[cfg(not(feature = "compression"))]
    let _ = options.decompressed_buffer;

    let mut cursor = cursor::Cursor::new_streaming(options.input);

    // Magic byte.
    let magic = cursor.take(1)?[0];
    if magic != ETF_MAGIC {
        return Err(EtfError::InvalidMagicNumber);
    }

    // Compression wrapper.
    if cursor.data.first() == Some(&tags::COMPRESSED) {
        #[cfg(not(feature = "compression"))]
        {
            let _ = cursor;
            return Err(EtfError::UnsupportedTag(tags::COMPRESSED));
        }

        #[cfg(feature = "compression")]
        {
            cursor.take(1)?; // consume tag 80
            let uncompressed_size = cursor.read_u32()? as usize;

            let decomp_buf = options
                .decompressed_buffer
                .ok_or(EtfError::InsufficientDecompressionBuffer)?;

            if decomp_buf.len() < uncompressed_size {
                return Err(EtfError::InsufficientDecompressionBuffer);
            }

            let target_buf = &mut decomp_buf[..uncompressed_size];

            let (_, rc) = zlib_rs::decompress_slice(target_buf, cursor.data, Default::default());
            if rc != zlib_rs::ReturnCode::Ok {
                return Err(EtfError::DecompressionFailed);
            }

            let mut dec_cursor = cursor::Cursor::new(target_buf);
            let mut arena =
                arena::Bump::new(options.ast_arena, options.limits.max_depth + 1, &options.limits);

            parser::parse_term(&mut dec_cursor, &mut arena)
        }
    } else {
        let mut arena =
            arena::Bump::new(options.ast_arena, options.limits.max_depth + 1, &options.limits);

        parser::parse_term(&mut cursor, &mut arena)
    }
}
