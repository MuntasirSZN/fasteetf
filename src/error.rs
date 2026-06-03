// ─────────────────────────────────────────────────────────────────────────────
// Errors returned during ETF parsing.
//
// Each variant corresponds to a class of spec violation described in:
// https://www.erlang.org/doc/apps/erts/erl_ext_dist
//
// The enum is kept small and flat so that fuzzers can easily track coverage
// and callers can match individual cases without `Box<dyn Error>` overhead.
//
// Display and Error impls are derived via `thiserror`:
//   • `no_std` builds             — only `core::fmt::Display` is derived.
//   • `std` (default) builds      — both `Display` and `std::error::Error`
//                                   are derived.
//
// `thiserror` decides which trait impls to emit based on its own `std`
// feature, which tracks our crate's `std` feature via Cargo.toml.
// ─────────────────────────────────────────────────────────────────────────────

use thiserror::Error;

/// Signals how many more bytes are needed to make progress during
/// incremental / streaming parsing.
///
/// Returned as part of [`EtfError::Incomplete`] when the input ends
/// before a complete term can be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Needed {
    /// Exactly `n` more bytes are required.
    ///
    /// This is a lower bound — the caller should obtain **at least** `n`
    /// additional bytes and re-attempt parsing from the beginning of the
    /// stream.
    #[error("need {0} more bytes")]
    Size(usize),

    /// The amount needed is not yet known.  The parser cannot make
    /// progress without more data, but it cannot compute exactly how
    /// many bytes are missing — obtain an arbitrary amount and retry.
    #[error("need more bytes (amount unknown)")]
    Unknown,
}

impl Needed {
    /// Returns the minimum number of additional bytes required, if known.
    #[inline]
    pub fn size(&self) -> Option<usize> {
        match self {
            Needed::Size(n) => Some(*n),
            Needed::Unknown => None,
        }
    }

    /// Returns `true` if the exact number of needed bytes is known.
    #[inline]
    pub fn is_exact(&self) -> bool {
        matches!(self, Needed::Size(_))
    }
}

/// Errors that can occur when decoding an ETF byte stream.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EtfError {
    /// The input ended before the expected number of bytes could be read,
    /// and the caller indicated no more data is coming.
    #[error("unexpected end of input")]
    UnexpectedEof,

    /// The input is incomplete — more bytes are needed before the parser
    /// can make progress.  Obtain additional data and re-parse from the
    /// beginning of the stream.
    #[error("incomplete input, {0}")]
    Incomplete(Needed),

    /// The first byte is not `131` (the ETF magic / version byte).
    #[error("invalid magic number (expected 131)")]
    InvalidMagicNumber,

    /// The input is compressed but no [`decompressed_buffer`] was supplied,
    /// or the supplied buffer is too small.
    ///
    /// [`decompressed_buffer`]: crate::ParseOptions::decompressed_buffer
    #[error("decompression buffer missing or too small")]
    InsufficientDecompressionBuffer,

    /// Zlib decompression of a [`COMPRESSED`] wrapper failed.
    ///
    /// [`COMPRESSED`]: crate::tags::COMPRESSED
    #[error("zlib decompression failed")]
    DecompressionFailed,

    /// The bump arena has been exhausted.  Increase [`ast_arena`] size.
    ///
    /// [`ast_arena`]: crate::ParseOptions::ast_arena
    #[error("parse arena exhausted")]
    ArenaExhausted,

    /// The term nesting depth exceeds the implementation limit.
    #[error("recursion depth limit exceeded")]
    RecursionLimitExceeded,

    /// A UTF-8 validation check on an atom or legacy float string failed.
    #[error("invalid UTF-8")]
    InvalidUtf8,

    /// A legacy [`FLOAT_EXT`] string could not be parsed as an `f64`.
    ///
    /// [`FLOAT_EXT`]: crate::tags::FLOAT_EXT
    #[error("invalid float encoding")]
    InvalidFloat,

    /// A length or size field contained a physically impossible value
    /// (e.g. a [`NEW_FUN_EXT`] size < 4).
    ///
    /// [`NEW_FUN_EXT`]: crate::tags::NEW_FUN_EXT
    #[error("invalid size field")]
    InvalidSize,

    /// A binary payload exceeds [`MAX_BINARY_SIZE`].
    ///
    /// [`MAX_BINARY_SIZE`]: crate::limits::MAX_BINARY_SIZE
    #[error("binary exceeds maximum size")]
    BinaryTooLarge,

    /// A list element count exceeds [`MAX_LIST_LEN`].
    ///
    /// [`MAX_LIST_LEN`]: crate::limits::MAX_LIST_LEN
    #[error("list exceeds maximum length")]
    ListTooLarge,

    /// A map arity exceeds [`MAX_MAP_LEN`].
    ///
    /// [`MAX_MAP_LEN`]: crate::limits::MAX_MAP_LEN
    #[error("map exceeds maximum arity")]
    MapTooLarge,

    /// An atom name exceeds [`MAX_ATOM_LEN`].
    ///
    /// [`MAX_ATOM_LEN`]: crate::limits::MAX_ATOM_LEN
    #[error("atom exceeds maximum length")]
    AtomTooLarge,

    /// The tag byte is not a recognised ETF term type (or is a known type
    /// that this parser does not yet handle).
    #[error("unsupported or invalid tag: {0}")]
    UnsupportedTag(u8),
}
