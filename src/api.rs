// ─────────────────────────────────────────────────────────────────────────────
// Public API: ParseOptions and parse functions.
//
// This module contains the main entry points for parsing ETF data:
// - `ParseOptions` struct for configuring parsing
// - `parse_etf` for parsing complete buffers
// - `parse_etf_streaming` for incremental/streaming parsing
// ─────────────────────────────────────────────────────────────────────────────

use core::mem::MaybeUninit;

use crate::arena;
use crate::cursor;
use crate::parser;
use crate::tags;
use crate::zlib;
use crate::EtfError;
use crate::Limits;

/// Options passed to [`parse_etf`].
pub struct ParseOptions<'a> {
    /// The raw ETF byte slice to parse.
    pub input: &'a [u8],
    /// Scratch space used by the bump arena to build the AST.  The required
    /// size depends on the complexity of the term; 8–16 kiB is a good starting
    /// point for most real-world messages.
    pub ast_arena: &'a mut [MaybeUninit<u8>],
    /// Resource limits enforced during parsing.
    ///
    /// Use [`Limits::default()`] for the built-in defaults, or construct a
    /// custom `Limits` with overridden fields for tighter/looser bounds.
    pub limits: Limits,
    /// An optional buffer used for decompression.  Must be large enough to
    /// hold the uncompressed data when the input is a compressed ETF stream.
    #[cfg(feature = "compression")]
    pub decompressed_buffer: Option<&'a mut [u8]>,
    /// Optional runtime zlib backend.  When `Some`, the supplied function
    /// is used to decompress any [`COMPRESSED`](tags::COMPRESSED) term
    /// regardless of the compile-time `zlib-*` feature.  When `None`, the
    /// compile-time backend (selected via the `zlib-rs`, `miniz_oxide`,
    /// `zlib`, `zlib-default`, `zlib-ng-compat`, `zlib-ng`, or
    /// `cloudflare-zlib` feature) is used.  If no backend is compiled in
    /// and this is `None`, encountering a compressed term yields
    /// [`EtfError::UnsupportedTag`].
    ///
    /// Pass `<MyBackend as ZlibBackend>::decompress` to use a custom
    /// implementation of the [`ZlibBackend`] trait.
    #[cfg(feature = "compression")]
    pub zlib_backend: Option<crate::ZlibDecompressFn>,
}

/// Parse an ETF-encoded term from a complete input buffer.
///
/// The wire format is:
///
/// ```text
/// 131 Tag Data…
/// ```
///
/// where `131` is the magic version byte ([`crate::ETF_MAGIC`]) and `Tag` identifies
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
/// The returned [`Term`](crate::Term) borrows from either the original `input` slice or
/// the `decompressed_buffer` — no heap allocation occurs.
///
/// ## Errors
///
/// Returns [`EtfError`] on malformed input, unsupported tags, arena
/// exhaustion, or decompression failure.
///
/// Spec: https://www.erlang.org/doc/apps/erts/erl_ext_dist
pub fn parse_etf<'a>(options: ParseOptions<'a>) -> Result<crate::Term<'a>, EtfError> {
    let mut cursor = cursor::Cursor::new(options.input);

    // Magic byte.
    let magic = cursor.take(1)?[0];
    if magic != crate::ETF_MAGIC {
        return Err(EtfError::InvalidMagicNumber);
    }

    // Hot path: not compressed.
    if cursor.data.first() != Some(&tags::COMPRESSED) {
        let mut arena = arena::Bump::new(options.ast_arena, &options.limits);
        let depth = 0usize;
        return parser::parse_term(&mut cursor, &mut arena, depth);
    }

    // Cold path: compression wrapper.
    #[cfg(not(feature = "compression"))]
    {
        Err(EtfError::UnsupportedTag(tags::COMPRESSED))
    }

    #[cfg(feature = "compression")]
    {
        cursor.take(1)?; // consume COMPRESSED tag
        let uncompressed_size = cursor.read_u32()? as usize;
        let decomp_buf = options
            .decompressed_buffer
            .ok_or(EtfError::InsufficientDecompressionBuffer)?;
        if decomp_buf.len() < uncompressed_size {
            return Err(EtfError::InsufficientDecompressionBuffer);
        }
        let target_buf = &mut decomp_buf[..uncompressed_size];
        zlib::decompress(target_buf, cursor.data, options.zlib_backend)?;
        let mut dec_cursor = cursor::Cursor::new(target_buf);
        let mut arena = arena::Bump::new(options.ast_arena, &options.limits);
        let depth = 0usize;
        parser::parse_term(&mut dec_cursor, &mut arena, depth)
    }
}

/// Parse an ETF-encoded term from a **potentially incomplete** input buffer.
///
/// This is the incremental / streaming entry point.  When the input does not
/// contain a complete term, this function returns
/// [`EtfError::Incomplete(Needed)`](EtfError::Incomplete) with the minimum number of additional
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
/// [`UnexpectedEof`](EtfError::UnexpectedEof) for truly truncated data, which is easier to distinguish
/// from a mere "need more data" signal.
pub fn parse_etf_streaming<'a>(options: ParseOptions<'a>) -> Result<crate::Term<'a>, EtfError> {
    let mut cursor = cursor::Cursor::new_streaming(options.input);

    // Magic byte.
    let magic = cursor.take(1)?[0];
    if magic != crate::ETF_MAGIC {
        return Err(EtfError::InvalidMagicNumber);
    }

    // Hot path: not compressed.
    if cursor.data.first() != Some(&tags::COMPRESSED) {
        let mut arena = arena::Bump::new(options.ast_arena, &options.limits);
        let depth = 0usize;
        return parser::parse_term(&mut cursor, &mut arena, depth);
    }

    // Cold path: compression wrapper.
    #[cfg(not(feature = "compression"))]
    {
        Err(EtfError::UnsupportedTag(tags::COMPRESSED))
    }

    #[cfg(feature = "compression")]
    {
        cursor.take(1)?; // consume COMPRESSED tag
        let uncompressed_size = cursor.read_u32()? as usize;
        let decomp_buf = options
            .decompressed_buffer
            .ok_or(EtfError::InsufficientDecompressionBuffer)?;
        if decomp_buf.len() < uncompressed_size {
            return Err(EtfError::InsufficientDecompressionBuffer);
        }
        let target_buf = &mut decomp_buf[..uncompressed_size];
        zlib::decompress(target_buf, cursor.data, options.zlib_backend)?;
        let mut dec_cursor = cursor::Cursor::new(target_buf);
        let mut arena = arena::Bump::new(options.ast_arena, &options.limits);
        let depth = 0usize;
        parser::parse_term(&mut dec_cursor, &mut arena, depth)
    }
}
