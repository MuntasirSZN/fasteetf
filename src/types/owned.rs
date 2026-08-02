// ─────────────────────────────────────────────────────────────────────────────
// Owned (heap-allocated) representations of ETF terms.
//
// Feature-gated behind `alloc` so that `no_std` users who don't need owned
// terms pay no code-size or dependency penalty.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
use crate::types::borrowed::{Function, Pid, Port, Record, Reference, Term};

/// Owned, heap-allocated equivalents of [`Term`] variants.
///
/// These types own their data and can outlive the original input buffer.
/// Conversion from the borrowed [`Term`] enum is provided via `From` impls.
#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
owned_wrapper!(PidOwned, Pid);
#[cfg(feature = "alloc")]
owned_wrapper!(PortOwned, Port);
#[cfg(feature = "alloc")]
owned_wrapper!(ReferenceOwned, Reference);
#[cfg(feature = "alloc")]
owned_wrapper!(RecordOwned, Record);

/// Owned version of [`Function`].
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct FunctionOwned(pub Vec<u8>);

#[cfg(feature = "alloc")]
impl<'a> From<Function<'a>> for FunctionOwned {
    #[inline]
    fn from(v: Function<'a>) -> Self {
        FunctionOwned(v.to_vec())
    }
}

#[cfg(feature = "alloc")]
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
            Term::List(elements) => OwnedTerm::List(elements.iter().map(|&t| t.into()).collect()),
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
            Term::Tuple(elements) => OwnedTerm::Tuple(elements.iter().map(|&t| t.into()).collect()),
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
