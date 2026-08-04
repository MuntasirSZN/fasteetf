// ─────────────────────────────────────────────────────────────────────────────
// Borrowed type definitions for decoded ETF terms.
//
// These types borrow from the input buffer and require no heap allocation.
// ─────────────────────────────────────────────────────────────────────────────

use core::hash::Hash;

use crate::simd::simd_eq;

/// A decoded Erlang term.  The lifetime `'a` is tied to the input buffer (or
/// the decompression buffer) — zero-copy, no heap allocation.
///
/// The spec-to-variant mapping is:
///
/// | Tag(s) | Variant |
/// |---|---|
/// | `SMALL_INTEGER_EXT`, `INTEGER_EXT` | [`Int`] |
/// | `SMALL_BIG_EXT` | [`BigInt`] |
/// | `LARGE_BIG_EXT` | [`BigInt`] |
/// | `NEW_FLOAT_EXT`, `FLOAT_EXT` | [`Float`] |
/// | `ATOM_UTF8_EXT`, `SMALL_ATOM_UTF8_EXT` | [`Atom`] |
/// | `SMALL_TUPLE_EXT`, `LARGE_TUPLE_EXT` | [`Tuple`] |
/// | `NIL_EXT` | [`List`]`(&[])` |
/// | `LIST_EXT` with `NIL_EXT` tail | [`List`] |
/// | `LIST_EXT` with non-nil tail | [`ImproperList`] |
/// | `STRING_EXT` | [`List`] (list of byte-sized `Int`s) |
/// | `BINARY_EXT` | [`Binary`] |
/// | `BIT_BINARY_EXT` | [`BitBinary`] |
/// | `MAP_EXT` | [`Map`] |
/// | `PID_EXT`, `NEW_PID_EXT` | [`Pid`] |
/// | `PORT_EXT`, `NEW_PORT_EXT`, `V4_PORT_EXT` | [`Port`] |
/// | `NEW_REFERENCE_EXT`, `NEWER_REFERENCE_EXT` | [`Ref`] |
/// | `NEW_FUN_EXT`, `EXPORT_EXT` | [`Function`] |
/// | `RECORD_EXT` | [`Record`] |
///
/// [`Int`]: Term::Int
/// [`BigInt`]: Term::BigInt
/// [`Float`]: Term::Float
/// [`Atom`]: Term::Atom
/// [`Tuple`]: Term::Tuple
/// [`List`]: Term::List
/// [`ImproperList`]: Term::ImproperList
/// [`Binary`]: Term::Binary
/// [`BitBinary`]: Term::BitBinary
/// [`Map`]: Term::Map
/// [`Pid`]: Term::Pid
/// [`Port`]: Term::Port
/// [`Ref`]: Term::Ref
/// [`Function`]: Term::Function
/// [`Record`]: Term::Record
#[derive(Debug, Clone, Copy)]
pub enum Term<'a> {
    /// A UTF-8 atom (spec: `ATOM_UTF8_EXT`, `SMALL_ATOM_UTF8_EXT`).
    Atom(AtomUtf8<'a>),
    /// A small signed integer (spec: `SMALL_INTEGER_EXT`, `INTEGER_EXT`).
    Int(i32),
    /// An arbitrary-precision integer (spec: `SMALL_BIG_EXT`, `LARGE_BIG_EXT`).
    /// The encoder auto-selects the appropriate tag based on digit count.
    BigInt {
        /// Sign byte: 0 = positive, 1 = negative.
        sign: u8,
        /// Big-endian digits (least significant byte first).
        digits: &'a [u8],
    },
    /// An IEEE 754 double-precision float (spec: `NEW_FLOAT_EXT`, `FLOAT_EXT`).
    Float(f64),
    /// A binary blob (spec: `BINARY_EXT`).
    Binary(&'a [u8]),
    /// A bitstring whose total bit-length may not be a multiple of 8
    /// (spec: `BIT_BINARY_EXT`).
    BitBinary {
        /// Number of significant bits in the last byte (1–8).
        bits: u8,
        /// Binary data padded to a whole number of bytes.
        data: &'a [u8],
    },
    /// A string (spec: `STRING_EXT`).
    String(&'a [u8]),
    /// A proper list (spec: `NIL_EXT` for empty, `LIST_EXT` with nil tail).
    List(&'a [Term<'a>]),
    /// An improper list (spec: `LIST_EXT` with non-nil tail).
    /// Stored as a single slice where the last element is the tail.
    ImproperList(&'a [Term<'a>]),
    /// A tuple (spec: `SMALL_TUPLE_EXT`, `LARGE_TUPLE_EXT`).
    Tuple(&'a [Term<'a>]),
    /// A map / dictionary (spec: `MAP_EXT`).
    Map(&'a [(Term<'a>, Term<'a>)]),
    /// A process identifier (spec: `PID_EXT`, `NEW_PID_EXT`).
    /// The slice includes the tag byte followed by the wire bytes.
    Pid(&'a [u8]),
    /// A port identifier (spec: `PORT_EXT`, `NEW_PORT_EXT`, `V4_PORT_EXT`).
    /// The slice includes the tag byte followed by the wire bytes.
    Port(&'a [u8]),
    /// A reference (spec: `NEW_REFERENCE_EXT`, `NEWER_REFERENCE_EXT`).
    /// The slice includes the tag byte followed by the wire bytes.
    Ref(&'a [u8]),
    /// A fun / function object (spec: `NEW_FUN_EXT`, `EXPORT_EXT`).
    /// The slice includes the tag byte followed by the wire bytes.
    Function(&'a [u8]),
    /// A native record (spec: `RECORD_EXT`, OTP 29.0).
    Record(&'a [u8]),
}

// ── Equality & hashing ──────────────────────────────────────────────────────
//
// Manual impls instead of derives because `Term::Float` holds an `f64`,
// and std deliberately omits `Eq`/`Hash` for floats (IEEE `PartialEq` is
// not reflexive for NaN, so no `Hash` can be consistent with it).
//
// We use *total* float semantics: `-0.0 == 0.0` (as IEEE requires) and all
// NaNs compare equal to each other.  This keeps `Eq` reflexive and makes
// `Hash` implementable.  The alternative — IEEE `PartialEq` with no
// `Eq`/`Hash` — would make `Term` unusable in `HashSet`/`HashMap`.

/// Total float equality: IEEE equality, except all NaNs compare equal.
#[inline]
pub(crate) fn float_eq(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

/// Hash for total float equality: `-0.0` normalizes to `0.0` and all NaNs
/// share one bit pattern.
#[inline]
pub(crate) fn hash_f64<H: core::hash::Hasher>(x: f64, state: &mut H) {
    let bits = if x == 0.0 {
        0u64 // both -0.0 and 0.0
    } else if x.is_nan() {
        f64::NAN.to_bits() // all NaNs hash identically
    } else {
        x.to_bits()
    };
    bits.hash(state);
}

impl<'a> PartialEq for Term<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Atom(a), Self::Atom(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (
                Self::BigInt { sign, digits },
                Self::BigInt {
                    sign: b_sign,
                    digits: b_digits,
                },
            ) => sign == b_sign && digits == b_digits,
            (Self::Float(a), Self::Float(b)) => float_eq(*a, *b),
            (Self::Binary(a), Self::Binary(b)) => a == b,
            (
                Self::BitBinary { bits, data },
                Self::BitBinary {
                    bits: b_bits,
                    data: b_data,
                },
            ) => bits == b_bits && data == b_data,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::ImproperList(a), Self::ImproperList(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Pid(a), Self::Pid(b)) => a == b,
            (Self::Port(a), Self::Port(b)) => a == b,
            (Self::Ref(a), Self::Ref(b)) => a == b,
            (Self::Function(a), Self::Function(b)) => a == b,
            (Self::Record(a), Self::Record(b)) => a == b,
            _ => false,
        }
    }
}

impl<'a> Eq for Term<'a> {}

impl<'a> core::hash::Hash for Term<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Atom(a) => a.hash(state),
            Self::Int(a) => a.hash(state),
            Self::BigInt { sign, digits } => {
                sign.hash(state);
                digits.hash(state);
            }
            Self::Float(a) => hash_f64(*a, state),
            Self::Binary(a) => a.hash(state),
            Self::BitBinary { bits, data } => {
                bits.hash(state);
                data.hash(state);
            }
            Self::String(a) => a.hash(state),
            Self::List(a) => a.hash(state),
            Self::ImproperList(a) => a.hash(state),
            Self::Tuple(a) => a.hash(state),
            Self::Map(a) => a.hash(state),
            Self::Pid(a) => a.hash(state),
            Self::Port(a) => a.hash(state),
            Self::Ref(a) => a.hash(state),
            Self::Function(a) => a.hash(state),
            Self::Record(a) => a.hash(state),
        }
    }
}

/// A UTF-8 atom that defers validation.
///
/// The parser stores the raw byte slice without checking UTF-8.  Call
/// [`as_str`](Self::as_str) when you need a `&str`; this performs the
/// validation once.
///
/// This is a key optimisation: if the caller only needs to compare atoms
/// for equality (matching on known atoms like `'true'`, `'false'`,
/// `'undefined'`), they can compare the raw bytes directly without the
/// cost of UTF-8 validation.
#[derive(Debug, Clone, Copy)]
pub struct AtomUtf8<'a>(pub &'a [u8]);

impl<'a> core::hash::Hash for AtomUtf8<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<'a> AtomUtf8<'a> {
    /// Create a new `AtomUtf8` without validating UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bytes are valid UTF-8, OR only use
    /// [`as_bytes`](Self::as_bytes) and never [`as_str`](Self::as_str).
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    /// Validate UTF-8 and return the atom as a string slice.
    #[inline(always)]
    pub fn as_str(&self) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(self.0)
    }

    /// Return the raw bytes (no copy, no validation).
    #[inline(always)]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0
    }

    /// Byte length of the atom.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the atom is the empty atom `''`.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ── PartialEq / Eq helpers so callers can match `Atom("true")` etc. ─────────

impl<'a> PartialEq<&str> for AtomUtf8<'a> {
    #[inline(always)]
    fn eq(&self, other: &&str) -> bool {
        simd_eq(self.0, other.as_bytes())
    }
}

impl<'a> PartialEq<AtomUtf8<'a>> for &str {
    #[inline(always)]
    fn eq(&self, other: &AtomUtf8<'a>) -> bool {
        simd_eq(self.as_bytes(), other.0)
    }
}

// Manual PartialEq for AtomUtf8 vs AtomUtf8 to use SIMD
impl<'a> PartialEq for AtomUtf8<'a> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        simd_eq(self.0, other.0)
    }
}

impl<'a> Eq for AtomUtf8<'a> {}

impl<'a> From<&'a str> for AtomUtf8<'a> {
    #[inline(always)]
    fn from(s: &'a str) -> Self {
        // &str is guaranteed valid UTF-8, so this is safe.
        unsafe { Self::from_bytes_unchecked(s.as_bytes()) }
    }
}

impl<'a> From<AtomUtf8<'a>> for Term<'a> {
    #[inline(always)]
    fn from(atom: AtomUtf8<'a>) -> Self {
        Term::Atom(atom)
    }
}

impl<'a> From<&'a str> for Term<'a> {
    #[inline(always)]
    fn from(s: &'a str) -> Self {
        Term::Atom(AtomUtf8::from(s))
    }
}

// ── Opaque identifier wrappers ──────────────────────────────────────────────
//
// These wrap the raw wire-format bytes for the corresponding types.  The
// caller can inspect or decode the fields further as needed.
//
// NOTE: These types are now just type aliases for &[u8] since the tag byte
// is folded into the slice for memory efficiency.

/// Opaque wrapper for an Erlang process identifier (PID).
///
/// The slice includes the ETF tag byte (`PID_EXT`=103 or `NEW_PID_EXT`=88)
/// followed by the wire bytes.
pub type Pid<'a> = &'a [u8];

/// Opaque wrapper for an Erlang port identifier.
pub type Port<'a> = &'a [u8];

/// Opaque wrapper for an Erlang reference.
pub type Reference<'a> = &'a [u8];

/// Opaque wrapper for an Erlang fun (function object).
pub type Function<'a> = &'a [u8];

/// Opaque wrapper for an Erlang native record (OTP 29.0).
pub type Record<'a> = &'a [u8];
