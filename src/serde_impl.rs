// ─────────────────────────────────────────────────────────────────────────────
// serde Serialize / Deserialize for ETF term types
//
// Feature-gated behind `serde` (implies `alloc`).
// ─────────────────────────────────────────────────────────────────────────────

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use serde_core::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_core::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, Serializer,
};

use crate::types::owned::{OwnedTerm, RecordOwned};
use crate::types::{AtomUtf8, Record, Term};

// ═════════════════════════════════════════════════════════════════════════════
//  Serialize
// ═════════════════════════════════════════════════════════════════════════════

impl<'a> Serialize for Term<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Term::Int(v) => serializer.serialize_i32(*v),
            Term::SmallBigInt { sign, digits } => {
                let mut s = serializer.serialize_struct("SmallBigInt", 2)?;
                s.serialize_field("sign", sign)?;
                s.serialize_field("digits", digits)?;
                s.end()
            }
            Term::LargeBigInt { sign, digits } => {
                let mut s = serializer.serialize_struct("LargeBigInt", 2)?;
                s.serialize_field("sign", sign)?;
                s.serialize_field("digits", digits)?;
                s.end()
            }
            Term::Float(v) => serializer.serialize_f64(*v),
            Term::Atom(a) => a.serialize(serializer),
            Term::Binary(data) => serializer.serialize_bytes(data),
            Term::BitBinary { bits, data } => {
                let mut s = serializer.serialize_struct("BitBinary", 2)?;
                s.serialize_field("bits", bits)?;
                s.serialize_field("data", data)?;
                s.end()
            }
            Term::List(elements) => {
                let mut seq = serializer.serialize_seq(Some(elements.len()))?;
                for elem in elements.iter() {
                    seq.serialize_element(elem)?;
                }
                seq.end()
            }
            Term::ImproperList { elements, tail } => {
                let mut s = serializer.serialize_struct("ImproperList", 2)?;
                s.serialize_field("elements", elements)?;
                s.serialize_field("tail", tail)?;
                s.end()
            }
            Term::Tuple(elements) => {
                let mut tup = serializer.serialize_tuple(elements.len())?;
                for elem in elements.iter() {
                    tup.serialize_element(elem)?;
                }
                tup.end()
            }
            Term::Map(pairs) => {
                let mut map = serializer.serialize_map(Some(pairs.len()))?;
                for (k, v) in pairs.iter() {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Term::Pid(p) => {
                let mut s = serializer.serialize_struct("Pid", 2)?;
                s.serialize_field("tag", &p.0)?;
                s.serialize_field("data", p.1)?;
                s.end()
            }
            Term::Port(p) => {
                let mut s = serializer.serialize_struct("Port", 2)?;
                s.serialize_field("tag", &p.0)?;
                s.serialize_field("data", p.1)?;
                s.end()
            }
            Term::Ref(r) => {
                let mut s = serializer.serialize_struct("Ref", 2)?;
                s.serialize_field("tag", &r.0)?;
                s.serialize_field("data", r.1)?;
                s.end()
            }
            Term::Function(f) => {
                let mut s = serializer.serialize_struct("Function", 2)?;
                s.serialize_field("tag", &f.0)?;
                s.serialize_field("data", f.1)?;
                s.end()
            }
            Term::Record(r) => serializer.serialize_bytes(r.0),
        }
    }
}

impl Serialize for OwnedTerm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            OwnedTerm::Int(v) => serializer.serialize_i32(*v),
            OwnedTerm::SmallBigInt { sign, digits } => {
                let mut s = serializer.serialize_struct("SmallBigInt", 2)?;
                s.serialize_field("sign", sign)?;
                s.serialize_field("digits", digits)?;
                s.end()
            }
            OwnedTerm::LargeBigInt { sign, digits } => {
                let mut s = serializer.serialize_struct("LargeBigInt", 2)?;
                s.serialize_field("sign", sign)?;
                s.serialize_field("digits", digits)?;
                s.end()
            }
            OwnedTerm::Float(v) => serializer.serialize_f64(*v),
            OwnedTerm::Atom(s) => serializer.serialize_str(s),
            OwnedTerm::Binary(data) => serializer.serialize_bytes(data),
            OwnedTerm::BitBinary { bits, data } => {
                let mut s = serializer.serialize_struct("BitBinary", 2)?;
                s.serialize_field("bits", bits)?;
                s.serialize_field("data", data)?;
                s.end()
            }
            OwnedTerm::List(elements) => {
                let mut seq = serializer.serialize_seq(Some(elements.len()))?;
                for elem in elements {
                    seq.serialize_element(elem)?;
                }
                seq.end()
            }
            OwnedTerm::ImproperList { elements, tail } => {
                let mut s = serializer.serialize_struct("ImproperList", 2)?;
                s.serialize_field("elements", elements)?;
                s.serialize_field("tail", tail)?;
                s.end()
            }
            OwnedTerm::Tuple(elements) => {
                let mut tup = serializer.serialize_tuple(elements.len())?;
                for elem in elements {
                    tup.serialize_element(elem)?;
                }
                tup.end()
            }
            OwnedTerm::Map(pairs) => {
                let mut map = serializer.serialize_map(Some(pairs.len()))?;
                for (k, v) in pairs {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            OwnedTerm::Pid(p) => {
                let mut s = serializer.serialize_struct("Pid", 2)?;
                s.serialize_field("tag", &p.0)?;
                s.serialize_field("data", &p.1)?;
                s.end()
            }
            OwnedTerm::Port(p) => {
                let mut s = serializer.serialize_struct("Port", 2)?;
                s.serialize_field("tag", &p.0)?;
                s.serialize_field("data", &p.1)?;
                s.end()
            }
            OwnedTerm::Ref(r) => {
                let mut s = serializer.serialize_struct("Ref", 2)?;
                s.serialize_field("tag", &r.0)?;
                s.serialize_field("data", &r.1)?;
                s.end()
            }
            OwnedTerm::Function(f) => {
                let mut s = serializer.serialize_struct("Function", 2)?;
                s.serialize_field("tag", &f.0)?;
                s.serialize_field("data", &f.1)?;
                s.end()
            }
            OwnedTerm::Record(r) => serializer.serialize_bytes(&r.0),
        }
    }
}

impl<'a> Serialize for AtomUtf8<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.as_str() {
            Ok(s) => serializer.serialize_str(s),
            Err(_) => serializer.serialize_bytes(self.as_bytes()),
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  Deserialize
// ═════════════════════════════════════════════════════════════════════════════

impl<'de> Deserialize<'de> for OwnedTerm {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OwnedTermVisitor)
    }
}

struct OwnedTermVisitor;

impl<'de> Visitor<'de> for OwnedTermVisitor {
    type Value = OwnedTerm;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any valid Erlang term representation")
    }

    #[inline]
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Atom(if v {
            "true".into()
        } else {
            "false".into()
        }))
    }

    #[inline]
    fn visit_i8<E: de::Error>(self, v: i8) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Int(v as i32))
    }

    #[inline]
    fn visit_i16<E: de::Error>(self, v: i16) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Int(v as i32))
    }

    #[inline]
    fn visit_i32<E: de::Error>(self, v: i32) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Int(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<OwnedTerm, E> {
        if let Ok(n) = i32::try_from(v) {
            Ok(OwnedTerm::Int(n))
        } else {
            let sign: u8 = if v < 0 { 1 } else { 0 };
            let abs = v.unsigned_abs();
            let digits = abs.to_le_bytes();
            let len = digits
                .iter()
                .rposition(|&b| b != 0)
                .map(|i| i + 1)
                .unwrap_or(1);
            Ok(OwnedTerm::SmallBigInt {
                sign,
                digits: digits[..len].to_vec(),
            })
        }
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Int(v as i32))
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Int(v as i32))
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<OwnedTerm, E> {
        if let Ok(n) = i32::try_from(v) {
            Ok(OwnedTerm::Int(n))
        } else {
            let digits = v.to_le_bytes().to_vec();
            Ok(OwnedTerm::SmallBigInt { sign: 0, digits })
        }
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<OwnedTerm, E> {
        if let Ok(n) = i32::try_from(v) {
            Ok(OwnedTerm::Int(n))
        } else {
            let digits = v.to_le_bytes().to_vec();
            Ok(OwnedTerm::SmallBigInt { sign: 0, digits })
        }
    }

    #[inline]
    fn visit_f32<E: de::Error>(self, v: f32) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Float(v as f64))
    }

    #[inline]
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Float(v))
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Atom(v.into()))
    }

    #[inline]
    fn visit_string<E: de::Error>(self, v: String) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Atom(v))
    }

    #[inline]
    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Binary(v.to_vec()))
    }

    #[inline]
    fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Binary(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OwnedTerm, A::Error> {
        let mut elements: Vec<OwnedTerm> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(elem) = seq.next_element()? {
            elements.push(elem);
        }
        Ok(OwnedTerm::List(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<OwnedTerm, A::Error> {
        let mut pairs: Vec<(OwnedTerm, OwnedTerm)> =
            Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry()? {
            pairs.push((key, value));
        }
        Ok(OwnedTerm::Map(pairs))
    }

    #[inline]
    fn visit_unit<E: de::Error>(self) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::List(Vec::new()))
    }

    #[inline]
    fn visit_none<E: de::Error>(self) -> Result<OwnedTerm, E> {
        Ok(OwnedTerm::Atom("undefined".into()))
    }

    #[inline]
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<OwnedTerm, D::Error> {
        OwnedTerm::deserialize(deserializer)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<OwnedTerm, D::Error> {
        OwnedTerm::deserialize(deserializer)
    }
}

// ── Opaque wrapper serde impls ─────────────────────────────────────────────

macro_rules! opaque_serde {
    ($name:ident, $owned:ident) => {
        impl<'a> Serialize for $name<'a> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_bytes(self.0)
            }
        }

        impl Serialize for $owned {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_bytes(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $owned {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.deserialize_bytes(UntaggedOpaqueVisitor::<$owned>(PhantomData))
            }
        }
    };
}

/// Shared visitor for untagged opaque wrappers (Record, RecordOwned).
struct UntaggedOpaqueVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for UntaggedOpaqueVisitor<T>
where
    T: OpaqueFromVec,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("byte array")
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<T, E> {
        Ok(T::from_vec(v.to_vec()))
    }

    fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<T, E> {
        Ok(T::from_vec(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<T, A::Error> {
        let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(b) = seq.next_element::<u8>()? {
            bytes.push(b);
        }
        Ok(T::from_vec(bytes))
    }
}

/// Helper trait so the generic visitor can construct opaque wrappers.
trait OpaqueFromVec {
    fn from_vec(v: Vec<u8>) -> Self;
}

impl OpaqueFromVec for RecordOwned {
    fn from_vec(v: Vec<u8>) -> Self {
        RecordOwned(v)
    }
}

opaque_serde!(Record, RecordOwned);

// ── Tagged opaque wrappers (Pid, Port, Reference, Function) ───────────────

/// Shared visitor for tagged opaque wrappers (Pid, Port, Reference, Function).
struct TaggedOpaqueVisitor;

impl<'de> Visitor<'de> for TaggedOpaqueVisitor {
    type Value = (u8, Vec<u8>);

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a struct with tag (u8) and data (byte array)")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(u8, Vec<u8>), A::Error> {
        let mut tag: Option<u8> = None;
        let mut data: Option<Vec<u8>> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "tag" => tag = Some(map.next_value()?),
                "data" => data = Some(map.next_value()?),
                other => {
                    let _ = map.next_value::<de::IgnoredAny>()?;
                    return Err(de::Error::unknown_field(other, &["tag", "data"]));
                }
            }
        }
        let tag = tag.ok_or_else(|| de::Error::missing_field("tag"))?;
        let data = data.ok_or_else(|| de::Error::missing_field("data"))?;
        Ok((tag, data))
    }
}

macro_rules! impl_tagged_deser {
    ($owned:ty) => {
        impl<'de> Deserialize<'de> for $owned {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let (tag, data) = deserializer.deserialize_any(TaggedOpaqueVisitor)?;
                Ok(Self(tag, data))
            }
        }
    };
}

impl_tagged_deser!(crate::types::owned::PidOwned);
impl_tagged_deser!(crate::types::owned::PortOwned);
impl_tagged_deser!(crate::types::owned::ReferenceOwned);
impl_tagged_deser!(crate::types::owned::FunctionOwned);

// Because the opaque_serde_tagged! macro defines Serialize, but
// impl_tagged_deser! provides Deserialize separately, we need to
// define Serialize impls explicitly:

impl<'a> Serialize for crate::types::Pid<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Pid", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", self.1)?;
        s.end()
    }
}

impl Serialize for crate::types::owned::PidOwned {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Pid", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", &self.1)?;
        s.end()
    }
}

impl<'a> Serialize for crate::types::Port<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Port", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", self.1)?;
        s.end()
    }
}

impl Serialize for crate::types::owned::PortOwned {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Port", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", &self.1)?;
        s.end()
    }
}

impl<'a> Serialize for crate::types::Reference<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Ref", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", self.1)?;
        s.end()
    }
}

impl Serialize for crate::types::owned::ReferenceOwned {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Ref", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", &self.1)?;
        s.end()
    }
}

impl<'a> Serialize for crate::types::Function<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Function", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", self.1)?;
        s.end()
    }
}

impl Serialize for crate::types::owned::FunctionOwned {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Function", 2)?;
        s.serialize_field("tag", &self.0)?;
        s.serialize_field("data", &self.1)?;
        s.end()
    }
}
