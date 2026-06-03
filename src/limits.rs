// ─────────────────────────────────────────────────────────────────────────────
// Hard limits — protection against memory-exhaustion and resource-exhaustion
// attacks from untrusted ETF input.
//
// Every length/arity/count read from the wire is checked against the
// corresponding limit below BEFORE the parser commits to processing the
// data.  This ensures predictable resource usage even for pathological
// inputs.
//
// Limits are configurable at runtime via [`Limits`].  The `MAX_*` constants
// below are the defaults used by `Limits::default()`.
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum byte length of a `BINARY_EXT` payload (64 MiB).
///
/// Spec: 109 Len(4) Data[Len]
pub const MAX_BINARY_SIZE: usize = 64 * 1024 * 1024;

/// Maximum byte length of a `BIT_BINARY_EXT` payload (64 MiB).
///
/// Spec: 77 Len(4) Bits(1) Data[Len]
pub const MAX_BIT_BINARY_SIZE: usize = 64 * 1024 * 1024;

/// Maximum number of elements in a `LIST_EXT` proper list.
///
/// Spec: 108 Len(4) Elements[Len] Tail
pub const MAX_LIST_LEN: usize = 1_000_000;

/// Maximum number of key-value pairs in a `MAP_EXT`.
///
/// Spec: 116 Arity(4) K1 V1 … Kn Vn
pub const MAX_MAP_LEN: usize = 1_000_000;

/// Maximum byte length for an atom name.
///
/// `ATOM_UTF8_EXT` already uses a 2-byte length (max 65535), and
/// `SMALL_ATOM_UTF8_EXT` uses a 1-byte length (max 255).  This limit
/// applies uniformly as a safety net.
///
/// Spec: 118 Len(2) AtomName[Len], 119 Len(1) AtomName[Len]
pub const MAX_ATOM_LEN: usize = 65_535;

/// Maximum arity for a `SMALL_TUPLE_EXT` / `LARGE_TUPLE_EXT`.
///
/// Spec: 104 Arity(1) Elem[Arity], 105 Arity(4) Elem[Arity]
pub const MAX_TUPLE_ARITY: usize = 1_000_000;

/// Maximum number of elements in a `STRING_EXT` (already bounded by the
/// 2-byte length field, but we check explicitly for defence in depth).
///
/// Spec: 107 Len(2) Characters[Len]
pub const MAX_STRING_LEN: usize = 65_535;

/// Maximum number of ID words for a reference.
///
/// `NEWER_REFERENCE_EXT` supports up to 5 words (since OTP 26).
///
/// Spec: 114 Len(2) Node Creation(1) ID[Len×4],
///       90  Len(2) Node Creation(4) ID[Len×4]
pub const MAX_REFERENCE_WORDS: usize = 5;

/// Maximum nesting depth of compound terms.
///
/// Mirrors the implicit limit in the Erlang runtime.
pub const MAX_DEPTH: usize = 128;

/// Maximum byte size of a `NEW_FUN_EXT` payload (64 MiB).
///
/// The `Size` field is a 4-byte big-endian unsigned that includes the 4
/// bytes of `Size` itself.  After subtracting 4, the remaining bytes are
/// the function encoding.
///
/// Spec: 112 Size(4) Arity(1) Uniq(16) Index(4) NumFree(4) …
pub const MAX_FUN_SIZE: usize = 64 * 1024 * 1024;

// ── Runtime-configurable limits ────────────────────────────────────────────

/// Resource limits enforced during ETF parsing.
///
/// Every length, arity, or count field read from the wire is checked
/// against the corresponding field **before** the parser touches the
/// variable-length payload.  This prevents allocation bombs, stack
/// overflows, and other resource-exhaustion attacks.
///
/// # Defaults
///
/// The [`Default`] impl returns the same values as the `MAX_*` constants
/// in this module:
///
/// | Field | Default |
/// |---|---|
/// | [`max_binary_size`](Self::max_binary_size) | 64 MiB |
/// | [`max_bit_binary_size`](Self::max_bit_binary_size) | 64 MiB |
/// | [`max_list_len`](Self::max_list_len) | 1 000 000 |
/// | [`max_map_len`](Self::max_map_len) | 1 000 000 |
/// | [`max_atom_len`](Self::max_atom_len) | 65 535 |
/// | [`max_tuple_arity`](Self::max_tuple_arity) | 1 000 000 |
/// | [`max_string_len`](Self::max_string_len) | 65 535 |
/// | [`max_reference_words`](Self::max_reference_words) | 5 |
/// | [`max_depth`](Self::max_depth) | 128 |
/// | [`max_fun_size`](Self::max_fun_size) | 64 MiB |
///
/// # Example — relaxing the binary limit for large payloads
///
/// ```ignore
/// use fasteetf::Limits;
///
/// let limits = Limits {
///     max_binary_size: 256 * 1024 * 1024,   // 256 MiB
///     ..Limits::default()
/// };
/// ```
///
/// # Example — tightening limits for embedded / low-memory targets
///
/// ```ignore
/// use fasteetf::Limits;
///
/// let limits = Limits {
///     max_list_len: 1024,
///     max_depth: 32,
///     max_binary_size: 65536,
///     ..Limits::default()
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum byte length of a `BINARY_EXT` payload.
    pub max_binary_size: usize,

    /// Maximum byte length of a `BIT_BINARY_EXT` payload.
    pub max_bit_binary_size: usize,

    /// Maximum number of elements in a `LIST_EXT` proper list.
    pub max_list_len: usize,

    /// Maximum number of key-value pairs in a `MAP_EXT`.
    pub max_map_len: usize,

    /// Maximum byte length for an atom name.
    pub max_atom_len: usize,

    /// Maximum arity for a `SMALL_TUPLE_EXT` / `LARGE_TUPLE_EXT`.
    pub max_tuple_arity: usize,

    /// Maximum number of elements in a `STRING_EXT`.
    pub max_string_len: usize,

    /// Maximum number of ID words for a reference.
    pub max_reference_words: usize,

    /// Maximum nesting depth of compound terms.
    pub max_depth: usize,

    /// Maximum byte size of a `NEW_FUN_EXT` payload.
    pub max_fun_size: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_binary_size: MAX_BINARY_SIZE,
            max_bit_binary_size: MAX_BIT_BINARY_SIZE,
            max_list_len: MAX_LIST_LEN,
            max_map_len: MAX_MAP_LEN,
            max_atom_len: MAX_ATOM_LEN,
            max_tuple_arity: MAX_TUPLE_ARITY,
            max_string_len: MAX_STRING_LEN,
            max_reference_words: MAX_REFERENCE_WORDS,
            max_depth: MAX_DEPTH,
            max_fun_size: MAX_FUN_SIZE,
        }
    }
}
