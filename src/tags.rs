// ─────────────────────────────────────────────────────────────────────────────
// Erlang External Term Format — Tag Constants
//
// Source: https://www.erlang.org/doc/apps/erts/erl_ext_dist
//
// Every tag defined in the specification is listed below, grouped by the
// section ordering used in the spec document.  Tags that are only relevant
// for the Erlang distribution protocol (not standalone term encoding) are
// listed separately.
// ─────────────────────────────────────────────────────────────────────────────

// ── Atoms ───────────────────────────────────────────────────────────────────

/// **ATOM_UTF8_EXT** (118) – UTF-8 atom, 2-byte length.
///
/// Wire format: `118 Len AtomName`
///   - `Len`      : 2-byte big-endian unsigned length.
///   - `AtomName` : `Len` bytes of UTF-8 text.
pub const ATOM_UTF8_EXT: u8 = 118;

/// **SMALL_ATOM_UTF8_EXT** (119) – UTF-8 atom, 1-byte length.
///
/// Wire format: `119 Len AtomName`
///   - `Len`      : 1-byte unsigned length.
///   - `AtomName` : `Len` bytes of UTF-8 text.
pub const SMALL_ATOM_UTF8_EXT: u8 = 119;

// ── Integers ────────────────────────────────────────────────────────────────

/// **SMALL_INTEGER_EXT** (97) – 1-byte unsigned integer.
///
/// Wire format: `97 Int`
///   - `Int` : 1-byte unsigned integer (range 0–255).
pub const SMALL_INTEGER_EXT: u8 = 97;

/// **INTEGER_EXT** (98) – 4-byte signed integer, big-endian.
///
/// Wire format: `98 Int`
///   - `Int` : 4-byte big-endian signed integer (range -2³¹ … 2³¹-1).
pub const INTEGER_EXT: u8 = 98;

// ── Bignums (arbitrary-precision integers) ──────────────────────────────────

/// **SMALL_BIG_EXT** (110) – Bignum with 1-byte digit count.
///
/// Wire format: `110 n Sign d₀ … dₙ₋₁`
///   - `n`    : 1-byte unsigned — number of following digit bytes.
///   - `Sign` : 0 = positive, 1 = negative.
///   - `dᵢ`   : `n` bytes in little-endian order (base 256).
///
/// Value = Σ(dᵢ × 256ⁱ).
pub const SMALL_BIG_EXT: u8 = 110;

/// **LARGE_BIG_EXT** (111) – Bignum with 4-byte digit count.
///
/// Wire format: `111 n Sign d₀ … dₙ₋₁`
///   - `n`    : 4-byte big-endian unsigned — number of following digit bytes.
///   - `Sign` : 0 = positive, 1 = negative.
///   - `dᵢ`   : `n` bytes in little-endian order (base 256).
pub const LARGE_BIG_EXT: u8 = 111;

// ── Floats ──────────────────────────────────────────────────────────────────

/// **NEW_FLOAT_EXT** (70) – IEEE 754 binary64 (OTP 17+, minor version 1).
///
/// Wire format: `70 IEEE_float`
///   - `IEEE_float` : 8-byte big-endian IEEE 754 double-precision.
pub const NEW_FLOAT_EXT: u8 = 70;

/// **FLOAT_EXT** (99) – Legacy string-form float (minor version 0).
///
/// Wire format: `99 Float_string`
///   - `Float_string` : 31 bytes — null-terminated ASCII produced by
///     `sprintf("%.20e", value)`.
///
/// Superseded by `NEW_FLOAT_EXT` since OTP 17.
pub const FLOAT_EXT: u8 = 99;

// ── Tuples ──────────────────────────────────────────────────────────────────

/// **SMALL_TUPLE_EXT** (104) – Tuple with 1-byte arity.
///
/// Wire format: `104 Arity Elem₁ … Elemₙ`
///   - `Arity` : 1-byte unsigned — number of elements.
///   - `Elemᵢ` : `Arity` recursively encoded terms.
pub const SMALL_TUPLE_EXT: u8 = 104;

/// **LARGE_TUPLE_EXT** (105) – Tuple with 4-byte arity.
///
/// Wire format: `105 Arity Elem₁ … Elemₙ`
///   - `Arity` : 4-byte big-endian unsigned — number of elements.
///   - `Elemᵢ` : `Arity` recursively encoded terms.
pub const LARGE_TUPLE_EXT: u8 = 105;

// ── Lists (proper, improper, nil, string optimisation) ──────────────────────

/// **NIL_EXT** (106) – Empty list `[]`.
///
/// Wire format: `106`
///   - No data follows.
pub const NIL_EXT: u8 = 106;

/// **STRING_EXT** (107) – Optimisation for lists of bytes (0–255).
///
/// Wire format: `107 Length Characters`
///   - `Length`     : 2-byte big-endian unsigned (max 65535).
///   - `Characters` : `Length` bytes, each representing one integer element.
///
/// Lists longer than 65535 elements must be encoded as `LIST_EXT`.
pub const STRING_EXT: u8 = 107;

/// **LIST_EXT** (108) – General list (proper or improper).
///
/// Wire format: `108 Length Elements Tail`
///   - `Length`   : 4-byte big-endian unsigned — number of elements.
///   - `Elements` : `Length` recursively encoded terms.
///   - `Tail`     : One encoded term — `NIL_EXT` for a proper list, or any
///     term for an improper list (e.g. `[a | b]`).
pub const LIST_EXT: u8 = 108;

// ── Maps ────────────────────────────────────────────────────────────────────

/// **MAP_EXT** (116) – Map / dictionary (OTP 17+).
///
/// Wire format: `116 Arity K₁ V₁ … Kₙ Vₙ`
///   - `Arity` : 4-byte big-endian unsigned — number of key-value pairs.
///   - `Kᵢ`    : Key (recursively encoded term).
///   - `Vᵢ`    : Value (recursively encoded term).
///
/// Duplicate keys are not allowed.
pub const MAP_EXT: u8 = 116;

// ── Binaries ────────────────────────────────────────────────────────────────

/// **BINARY_EXT** (109) – Raw binary, 4-byte length.
///
/// Wire format: `109 Len Data`
///   - `Len`  : 4-byte big-endian unsigned.
///   - `Data` : `Len` bytes of raw data.
pub const BINARY_EXT: u8 = 109;

/// **BIT_BINARY_EXT** (77) – Bitstring (length in bits not necessarily
/// a multiple of 8).
///
/// Wire format: `77 Len Bits Data`
///   - `Len`  : 4-byte big-endian unsigned — byte-length of `Data`.
///   - `Bits` : 1 byte — number of *used* bits in the last byte (1–8).
///     A value of 0 means the last byte is fully used.
///   - `Data` : `Len` bytes of packed binary data.
pub const BIT_BINARY_EXT: u8 = 77;

// ── Process identifiers ─────────────────────────────────────────────────────

/// **PID_EXT** (103) – PID with 1-byte Creation (pre-OTP-23).
///
/// Wire format: `103 Node ID Serial Creation`
///   - `Node`     : Encoded atom.
///   - `ID`       : 4-byte big-endian unsigned.
///   - `Serial`   : 4-byte big-endian unsigned.
///   - `Creation` : 1 byte — only 2 bits significant.
///
/// Superseded by `NEW_PID_EXT` since OTP 23.
pub const PID_EXT: u8 = 103;

/// **NEW_PID_EXT** (88) – PID with 4-byte Creation (OTP 19+).
///
/// Wire format: `88 Node ID Serial Creation`
///   - `Node`     : Encoded atom.
///   - `ID`       : 4-byte big-endian unsigned.
///   - `Serial`   : 4-byte big-endian unsigned.
///   - `Creation` : 4-byte big-endian unsigned (≥ 1).
///
/// Mandatory since OTP 23 (`DFLAG_BIG_CREATION`).
pub const NEW_PID_EXT: u8 = 88;

// ── Ports ───────────────────────────────────────────────────────────────────

/// **PORT_EXT** (102) – Port with 1-byte Creation (pre-OTP-23).
///
/// Wire format: `102 Node ID Creation`
///   - `Node`     : Encoded atom.
///   - `ID`       : 4-byte big-endian unsigned.
///   - `Creation` : 1 byte — only 2 bits significant.
///
/// Superseded by `NEW_PORT_EXT` since OTP 23.
pub const PORT_EXT: u8 = 102;

/// **NEW_PORT_EXT** (89) – Port with 4-byte Creation (OTP 19+).
///
/// Wire format: `89 Node ID Creation`
///   - `Node`     : Encoded atom.
///   - `ID`       : 4-byte big-endian unsigned (28 bits significant).
///   - `Creation` : 4-byte big-endian unsigned.
///
/// Mandatory since OTP 23 (`DFLAG_BIG_CREATION`).
pub const NEW_PORT_EXT: u8 = 89;

/// **V4_PORT_EXT** (120) – Port with 8-byte ID (OTP 26+).
///
/// Wire format: `120 Node ID Creation`
///   - `Node`     : Encoded atom.
///   - `ID`       : 8-byte big-endian unsigned (full 64-bit).
///   - `Creation` : 4-byte big-endian unsigned.
///
/// Mandatory since OTP 26 (`DFLAG_V4_NC`).
pub const V4_PORT_EXT: u8 = 120;

// ── References ──────────────────────────────────────────────────────────────

/// **NEW_REFERENCE_EXT** (114) – Multi-word reference (1-byte Creation).
///
/// Wire format: `114 Len Node Creation ID₁ … IDₗₑₙ`
///   - `Len`      : 2-byte big-endian unsigned.
///   - `Node`     : Encoded atom.
///   - `Creation` : 1 byte — only 2 bits significant.
///   - `IDᵢ`      : `Len` × 4-byte big-endian unsigned words.
///     First word: 18 bits significant; rest: 0.
pub const NEW_REFERENCE_EXT: u8 = 114;

/// **NEWER_REFERENCE_EXT** (90) – Reference with 4-byte Creation (OTP 19+).
///
/// Wire format: `90 Len Node Creation ID₁ … IDₗₑₙ`
///   - `Len`      : 2-byte big-endian unsigned (max 5).
///   - `Node`     : Encoded atom.
///   - `Creation` : 4-byte big-endian unsigned.
///   - `IDᵢ`      : `Len` × 4-byte big-endian unsigned words.
///
/// Mandatory since OTP 23 (`DFLAG_BIG_CREATION`).  Supports up to 5 ID
/// words since OTP 26 (`DFLAG_V4_NC`).
pub const NEWER_REFERENCE_EXT: u8 = 90;

// ── Functions ───────────────────────────────────────────────────────────────

/// **NEW_FUN_EXT** (112) – Internal fun `fun F/A` or anonymous fun.
///
/// Wire format: `112 Size Arity Uniq Index NumFree Module OldIndex OldUniq
///               Pid FreeVars`
///   - `Size`     : 4-byte big-endian — total bytes including `Size`.
///   - `Arity`    : 1 byte.
///   - `Uniq`     : 16 bytes — MD5 of significant parts of the BEAM file.
///   - `Index`    : 4-byte big-endian — unique index within the module.
///   - `NumFree`  : 4-byte big-endian — number of free variables.
///   - `Module`   : Encoded atom.
///   - `OldIndex` : Encoded integer (SMALL_INTEGER_EXT or INTEGER_EXT).
///   - `OldUniq`  : Encoded integer (SMALL_INTEGER_EXT or INTEGER_EXT).
///   - `Pid`      : Encoded PID.
///   - `FreeVars` : `NumFree` recursively encoded terms.
///
/// Parsed as an opaque binary carrying `Size – 4` bytes after the tag.
pub const NEW_FUN_EXT: u8 = 112;

/// **EXPORT_EXT** (113) – External fun `fun M:F/A`.
///
/// Wire format: `113 Module Function Arity`
///   - `Module`   : Encoded atom.
///   - `Function` : Encoded atom.
///   - `Arity`    : Encoded integer (SMALL_INTEGER_EXT).
///
/// Captured as an opaque byte range containing the three encoded sub-terms.
pub const EXPORT_EXT: u8 = 113;

// ── Records (OTP 29.0) ──────────────────────────────────────────────────────

/// **RECORD_EXT** (67) – Native record encoding (OTP 29.0).
///
/// Wire format: `67 #Fields Flags Module Name FieldNames₁ … FieldNamesₙ
///               Values₁ … Valuesₙ`
///   - `#Fields`    : 4-byte big-endian unsigned — number of fields.
///   - `Flags`      : 1 byte — LSB: 0 = unexported, 1 = exported.
///   - `Module`     : Encoded atom.
///   - `Name`       : Encoded atom.
///   - `FieldNames` : `#Fields` encoded atoms.
///   - `Values`     : `#Fields` recursively encoded terms.
pub const RECORD_EXT: u8 = 67;

// ── Special / other ─────────────────────────────────────────────────────────

/// **COMPRESSED** (80) – Zlib-compressed term.
///
/// Not a term tag *per se* — wraps the entire term stream:
///
/// Wire format: `131 80 UncompressedSize ZlibData`
///   - `UncompressedSize` : 4-byte big-endian unsigned.
///   - `ZlibData`         : Deflate-compressed payload (starts with a tag).
///
/// Handled by `parse_etf` before recursion begins.
pub const COMPRESSED: u8 = 80;

/// **ATOM_CACHE_REF** (82) – Reference to a cached atom in the distribution
/// header (distribution only — not a standalone term encoding).
///
/// Wire format: `82 AtomCacheReferenceIndex`
///   - `AtomCacheReferenceIndex` : 1 byte (0–254).
///
/// Distribution-only; never valid in standalone ETF.
pub const ATOM_CACHE_REF: u8 = 82;

/// **LOCAL_EXT** (121) – Marks an alternative local encoding (OTP 26+).
///
/// Wire format: `121 …` — opaque, decoder-specific.
///
/// Not decodable by a generic parser — the bytes that follow are in a
/// private format defined by the encoder.
pub const LOCAL_EXT: u8 = 121;
