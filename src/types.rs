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
    ///
    /// UTF-8 validation is deferred until [`AtomUtf8::as_str`] is called.
    Atom(AtomUtf8<'a>),
    /// A small signed integer (spec: `SMALL_INTEGER_EXT`, `INTEGER_EXT`).
    Int(i32),
    /// An arbitrary-precision integer with 1-byte digit count
    /// (spec: `SMALL_BIG_EXT`).
    SmallBigInt { sign: u8, digits: &'a [u8] },
    /// An arbitrary-precision integer with 4-byte digit count
    /// (spec: `LARGE_BIG_EXT`).
    LargeBigInt { sign: u8, digits: &'a [u8] },
    /// An IEEE 754 double-precision float (spec: `NEW_FLOAT_EXT`, `FLOAT_EXT`).
    Float(f64),
    /// A binary blob (spec: `BINARY_EXT`).
    Binary(&'a [u8]),
    /// A bitstring whose total bit-length may not be a multiple of 8
    /// (spec: `BIT_BINARY_EXT`).
    BitBinary { bits: u8, data: &'a [u8] },
    /// A proper list (spec: `NIL_EXT` for empty, `LIST_EXT` with nil tail).
    List(&'a [Term<'a>]),
    /// An improper list (spec: `LIST_EXT` with non-nil tail).
    ImproperList {
        elements: &'a [Term<'a>],
        tail: &'a Term<'a>,
    },
    /// A tuple (spec: `SMALL_TUPLE_EXT`, `LARGE_TUPLE_EXT`).
    Tuple(&'a [Term<'a>]),
    /// A map / dictionary (spec: `MAP_EXT`).
    Map(&'a [(Term<'a>, Term<'a>)]),
    /// A process identifier (spec: `PID_EXT`, `NEW_PID_EXT`).
    Pid(Pid<'a>),
    /// A port identifier (spec: `PORT_EXT`, `NEW_PORT_EXT`, `V4_PORT_EXT`).
    Port(Port<'a>),
    /// A reference (spec: `NEW_REFERENCE_EXT`, `NEWER_REFERENCE_EXT`).
    Ref(Reference<'a>),
    /// A fun / function object (spec: `NEW_FUN_EXT`, `EXPORT_EXT`).
    Function(Function<'a>),
    /// A native record (spec: `RECORD_EXT`, OTP 29.0).
    Record(Record<'a>),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtomUtf8<'a>(&'a [u8]);

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
        self.0 == other.as_bytes()
    }
}

impl<'a> PartialEq<AtomUtf8<'a>> for &str {
    #[inline(always)]
    fn eq(&self, other: &AtomUtf8<'a>) -> bool {
        self.as_bytes() == other.0
    }
}

// ── Opaque identifier wrappers ──────────────────────────────────────────────
//
// These wrap the raw wire-format bytes for the corresponding types.  The
// caller can inspect or decode the fields further as needed.

/// Opaque wrapper for an Erlang process identifier (PID).
///
/// The first element is the ETF tag byte (`PID_EXT`=103 or `NEW_PID_EXT`=88).
/// The second element is the raw wire bytes **after** the tag, including the
/// node atom and the fixed fields.
///
/// Wire format (PID_EXT): `103 Node(atom) ID(4) Serial(4) Creation(1)`
/// Wire format (NEW_PID_EXT): `88 Node(atom) ID(4) Serial(4) Creation(4)`
#[derive(Debug, Clone, Copy)]
pub struct Pid<'a>(pub u8, pub &'a [u8]);

/// Opaque wrapper for an Erlang port identifier.
///
/// The first element is the ETF tag byte (`PORT_EXT`=102, `NEW_PORT_EXT`=89,
/// or `V4_PORT_EXT`=120).
/// The second element is the raw wire bytes **after** the tag, including the
/// node atom and the fixed fields.
///
/// Wire format (PORT_EXT): `102 Node(atom) ID(4) Creation(1)`
/// Wire format (NEW_PORT_EXT): `89 Node(atom) ID(4) Creation(4)`
/// Wire format (V4_PORT_EXT): `120 Node(atom) ID(8) Creation(4)`
#[derive(Debug, Clone, Copy)]
pub struct Port<'a>(pub u8, pub &'a [u8]);

/// Opaque wrapper for an Erlang reference.
///
/// The first element is the ETF tag byte (`NEW_REFERENCE_EXT`=114 or
/// `NEWER_REFERENCE_EXT`=90).
/// The second element is the raw wire bytes **after** the tag, including the
/// len, node atom, creation, and ID words.
///
/// Wire format (NEW_REFERENCE_EXT): `114 Len(2) Node(atom) Creation(1) ID[len×4]`
/// Wire format (NEWER_REFERENCE_EXT): `90 Len(2) Node(atom) Creation(4) ID[len×4]`
#[derive(Debug, Clone, Copy)]
pub struct Reference<'a>(pub u8, pub &'a [u8]);

/// Opaque wrapper for an Erlang fun (function object).
///
/// The first element is the ETF tag byte (`NEW_FUN_EXT`=112 or
/// `EXPORT_EXT`=113).
/// The second element is the raw wire bytes **after** the tag.
///
/// Wire format (NEW_FUN_EXT): `112 Size(4) Arity(1) Uniq(16) Index(4)
/// NumFree(4) Module(atom) OldIndex(term) OldUniq(term) Pid(pid)
/// FreeVars(NumFree terms)` — stored bytes are **everything after Size**
/// (Arity … FreeVars).
///
/// Wire format (EXPORT_EXT): `113 Module(atom) Function(atom) Arity(int)`
/// — stored bytes are Module + Function + Arity.
#[derive(Debug, Clone, Copy)]
pub struct Function<'a>(pub u8, pub &'a [u8]);

/// Opaque wrapper for an Erlang native record (OTP 29.0).
///
/// The wrapped bytes span everything after the `RECORD_EXT` tag byte:
/// `#Fields(4) Flags(1) Module(atom) Name(atom) FieldNames(#Fields × atom)
///  Values(#Fields × term)`.
#[derive(Debug, Clone, Copy)]
pub struct Record<'a>(pub &'a [u8]);

// ── Owned (heap-allocated) representations ──────────────────────────────────
//
// Feature-gated behind `alloc` so that `no_std` users who don't need owned
// terms pay no code-size or dependency penalty.

#[cfg(feature = "alloc")]
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
        SmallBigInt { sign: u8, digits: Vec<u8> },
        /// A bignum with 4-byte digit count.
        LargeBigInt { sign: u8, digits: Vec<u8> },
        /// An IEEE 754 double-precision float.
        Float(f64),
        /// A binary blob.
        Binary(Vec<u8>),
        /// A bitstring.
        BitBinary { bits: u8, data: Vec<u8> },
        /// A proper list.
        List(Vec<OwnedTerm>),
        /// An improper list.
        ImproperList {
            elements: Vec<OwnedTerm>,
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
                    $name(v.0.to_vec())
                }
            }
        };
    }

    owned_wrapper!(RecordOwned, Record);

    /// Owned version of [`Function`].
    #[derive(Debug, Clone)]
    pub struct FunctionOwned(pub u8, pub Vec<u8>);

    impl<'a> From<Function<'a>> for FunctionOwned {
        #[inline]
        fn from(v: Function<'a>) -> Self {
            FunctionOwned(v.0, v.1.to_vec())
        }
    }

    /// Owned version of [`Pid`].
    #[derive(Debug, Clone)]
    pub struct PidOwned(pub u8, pub Vec<u8>);

    impl<'a> From<Pid<'a>> for PidOwned {
        #[inline]
        fn from(v: Pid<'a>) -> Self {
            PidOwned(v.0, v.1.to_vec())
        }
    }

    /// Owned version of [`Port`].
    #[derive(Debug, Clone)]
    pub struct PortOwned(pub u8, pub Vec<u8>);

    impl<'a> From<Port<'a>> for PortOwned {
        #[inline]
        fn from(v: Port<'a>) -> Self {
            PortOwned(v.0, v.1.to_vec())
        }
    }

    /// Owned version of [`Reference`].
    #[derive(Debug, Clone)]
    pub struct ReferenceOwned(pub u8, pub Vec<u8>);

    impl<'a> From<Reference<'a>> for ReferenceOwned {
        #[inline]
        fn from(v: Reference<'a>) -> Self {
            ReferenceOwned(v.0, v.1.to_vec())
        }
    }

    impl<'a> From<Term<'a>> for OwnedTerm {
        fn from(term: Term<'a>) -> Self {
            match term {
                Term::Atom(a) => {
                    // Use lossy decode so invalid-UTF-8 atoms don't prevent
                    // round-tripping through the owned form.
                    let s = String::from_utf8_lossy(a.as_bytes()).into_owned();
                    OwnedTerm::Atom(s)
                }
                Term::Int(i) => OwnedTerm::Int(i),
                Term::SmallBigInt { sign, digits } => OwnedTerm::SmallBigInt {
                    sign,
                    digits: digits.to_vec(),
                },
                Term::LargeBigInt { sign, digits } => OwnedTerm::LargeBigInt {
                    sign,
                    digits: digits.to_vec(),
                },
                Term::Float(f) => OwnedTerm::Float(f),
                Term::Binary(b) => OwnedTerm::Binary(b.to_vec()),
                Term::BitBinary { bits, data } => OwnedTerm::BitBinary {
                    bits,
                    data: data.to_vec(),
                },
                Term::List(elements) => {
                    OwnedTerm::List(elements.iter().map(|&t| t.into()).collect())
                }
                Term::ImproperList { elements, tail } => OwnedTerm::ImproperList {
                    elements: elements.iter().map(|&t| t.into()).collect(),
                    tail: Box::new((*tail).into()),
                },
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
            }
        }
    }
}
