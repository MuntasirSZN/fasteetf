use serde_core::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, Serializer,
};

use crate::types::owned::OwnedTerm;
use crate::types::{AtomUtf8, Term};

// ═════════════════════════════════════════════════════════════════════════════
//  Serialize
// ═════════════════════════════════════════════════════════════════════════════

impl<'a> Serialize for Term<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Term::Int(v) => serializer.serialize_i32(*v),
            Term::BigInt { sign, digits } => {
                let mut s = serializer.serialize_struct("BigInt", 2)?;
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
            Term::String(data) => serializer.serialize_bytes(data),
            Term::List(elements) => {
                let mut seq = serializer.serialize_seq(Some(elements.len()))?;
                for elem in elements.iter() {
                    seq.serialize_element(elem)?;
                }
                seq.end()
            }
            Term::ImproperList(elements) => {
                // ImproperList is a single slice where the last element is the tail
                let mut seq = serializer.serialize_seq(Some(elements.len()))?;
                for elem in elements.iter() {
                    seq.serialize_element(elem)?;
                }
                seq.end()
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
            Term::Pid(p) => serializer.serialize_bytes(p),
            Term::Port(p) => serializer.serialize_bytes(p),
            Term::Ref(r) => serializer.serialize_bytes(r),
            Term::Function(f) => serializer.serialize_bytes(f),
            Term::Record(r) => serializer.serialize_bytes(r),
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
            OwnedTerm::Pid(p) => serializer.serialize_bytes(&p.0),
            OwnedTerm::Port(p) => serializer.serialize_bytes(&p.0),
            OwnedTerm::Ref(r) => serializer.serialize_bytes(&r.0),
            OwnedTerm::Function(f) => serializer.serialize_bytes(&f.0),
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
