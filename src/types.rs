// ─────────────────────────────────────────────────────────────────────────────
// Type definitions for decoded ETF terms.
//
// Each variant in [`Term`] corresponds to one or more tag formats defined in:
// https://www.erlang.org/doc/apps/erts/erl_ext_dist
// ─────────────────────────────────────────────────────────────────────────────

/// A decoded Erlang term.  The lifetime `'a` is tied to the input buffer (or
/// the decompression buffer) — zero-copy, no heap allocation.
///
/// The spec-to-variant mapping is:
///
/// | Tag(s) | Variant |
/// |---|---|
/// | `SMALL_INTEGER_EXT`, `INTEGER_EXT` | [`Int`] |
/// | `SMALL_BIG_EXT` | [`SmallBigInt`] |
/// | `LARGE_BIG_EXT` | [`LargeBigInt`] |
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
/// [`SmallBigInt`]: Term::SmallBigInt
/// [`LargeBigInt`]: Term::LargeBigInt
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

// ── Lazy-UTF-8 atom ─────────────────────────────────────────────────────────

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
pub struct AtomUtf8<'a>(&'a [u8]);

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

use crate::simd::simd_eq;

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

// ── Owned (heap-allocated) representations ──────────────────────────────────
//
// Feature-gated behind `alloc` so that `no_std` users who don't need owned
// terms pay no code-size or dependency penalty.

#[cfg(feature = "alloc")]
/// Owned, heap-allocated equivalents of [`Term`] variants.
///
/// These types own their data and can outlive the original input buffer.
/// Conversion from the borrowed [`Term`] enum is provided via `From` impls.
pub mod owned {
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;

    use crate::types::*;

    /// An owned, heap-allocated Erlang term.
    ///
    /// This is useful when you need to store terms beyond the lifetime of
    /// the input buffer (e.g. in a cache, or when sending across threads).
    ///
    /// Conversion from [`Term`] is provided via `From`.
    #[derive(Debug, Clone)]
    pub enum OwnedTerm {
        /// A UTF-8 atom (lazily validated; invalid bytes use lossy decode).
        Atom(String),
        /// A small signed integer.
        Int(i32),
        /// A bignum with 1-byte digit count.
        SmallBigInt {
            /// Sign byte: 0 = positive, 1 = negative.
            sign: u8,
            /// Big-endian digits (least significant byte first).
            digits: Vec<u8>,
        },
        /// A bignum with 4-byte digit count.
        LargeBigInt {
            /// Sign byte: 0 = positive, 1 = negative.
            sign: u8,
            /// Big-endian digits (least significant byte first).
            digits: Vec<u8>,
        },
        /// An IEEE 754 double-precision float.
        Float(f64),
        /// A binary blob.
        Binary(Vec<u8>),
        /// A bitstring.
        BitBinary {
            /// Number of significant bits in the last byte (1–8).
            bits: u8,
            /// Binary data padded to a whole number of bytes.
            data: Vec<u8>,
        },
        /// A proper list.
        List(Vec<OwnedTerm>),
        /// An improper list.
        ImproperList {
            /// Prefix elements before the tail.
            elements: Vec<OwnedTerm>,
            /// Non-nil tail term.
            tail: Box<OwnedTerm>,
        },
        /// A tuple.
        Tuple(Vec<OwnedTerm>),
        /// A map.
        Map(Vec<(OwnedTerm, OwnedTerm)>),
        /// A process identifier.
        Pid(PidOwned),
        /// A port identifier.
        Port(PortOwned),
        /// A reference.
        Ref(ReferenceOwned),
        /// A function.
        Function(FunctionOwned),
        /// A record.
        Record(RecordOwned),
    }

    macro_rules! owned_wrapper {
        ($name:ident, $borrowed:ident) => {
            /// Owned version of the corresponding borrowed wrapper.
            #[derive(Debug, Clone)]
            pub struct $name(pub Vec<u8>);

            impl<'a> From<$borrowed<'a>> for $name {
                #[inline]
                fn from(v: $borrowed<'a>) -> Self {
                    $name(v.to_vec())
                }
            }
        };
    }

    owned_wrapper!(PidOwned, Pid);
    owned_wrapper!(PortOwned, Port);
    owned_wrapper!(ReferenceOwned, Reference);
    owned_wrapper!(RecordOwned, Record);

    /// Owned version of [`Function`].
    #[derive(Debug, Clone)]
    pub struct FunctionOwned(pub Vec<u8>);

    impl<'a> From<Function<'a>> for FunctionOwned {
        #[inline]
        fn from(v: Function<'a>) -> Self {
            FunctionOwned(v.to_vec())
        }
    }

    impl<'a> From<Term<'a>> for OwnedTerm {
        fn from(term: Term<'a>) -> Self {
            match term {
                Term::Atom(a) => {
                    let s = String::from_utf8_lossy(a.as_bytes()).into_owned();
                    OwnedTerm::Atom(s)
                }
                Term::Int(i) => OwnedTerm::Int(i),
                Term::BigInt { sign, digits } => {
                    if digits.len() > 255 {
                        OwnedTerm::LargeBigInt {
                            sign,
                            digits: digits.to_vec(),
                        }
                    } else {
                        OwnedTerm::SmallBigInt {
                            sign,
                            digits: digits.to_vec(),
                        }
                    }
                }
                Term::Float(f) => OwnedTerm::Float(f),
                Term::Binary(b) => OwnedTerm::Binary(b.to_vec()),
                Term::BitBinary { bits, data } => OwnedTerm::BitBinary {
                    bits,
                    data: data.to_vec(),
                },
                Term::List(elements) => {
                    OwnedTerm::List(elements.iter().map(|&t| t.into()).collect())
                }
                Term::ImproperList(elements) => {
                    // New representation: single slice with tail as last element
                    let len = elements.len();
                    if len < 2 {
                        return OwnedTerm::List(Vec::new());
                    }
                    let (prefix, tail) = elements.split_at(len - 1);
                    OwnedTerm::ImproperList {
                        elements: prefix.iter().map(|&t| t.into()).collect(),
                        tail: Box::new(tail[0].into()),
                    }
                }
                Term::Tuple(elements) => {
                    OwnedTerm::Tuple(elements.iter().map(|&t| t.into()).collect())
                }
                Term::Map(pairs) => {
                    OwnedTerm::Map(pairs.iter().map(|&(k, v)| (k.into(), v.into())).collect())
                }
                Term::Pid(p) => OwnedTerm::Pid(p.into()),
                Term::Port(p) => OwnedTerm::Port(p.into()),
                Term::Ref(r) => OwnedTerm::Ref(r.into()),
                Term::Function(f) => OwnedTerm::Function(f.into()),
                Term::Record(r) => OwnedTerm::Record(r.into()),
                Term::String(data) => OwnedTerm::Binary(data.to_vec()), // String is bytes, store as Binary
            }
        }
    }
}
