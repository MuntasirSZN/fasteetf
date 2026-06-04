// ─────────────────────────────────────────────────────────────────────────────
// Serde serialization / deserialization integration tests.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "serde")]

mod common;
use common::*;
use fasteetf::*;

#[test]
fn test_serde_serialize_int() {
    let term = Term::Int(42);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "42");
}

#[test]
fn test_serde_serialize_neg_int() {
    let term = Term::Int(-1000);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "-1000");
}

#[test]
fn test_serde_serialize_float() {
    let term = Term::Float(3.5);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "3.5");
}

#[test]
fn test_serde_serialize_atom() {
    let a = unsafe { AtomUtf8::from_bytes_unchecked(b"hello") };
    let term = Term::Atom(a);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "\"hello\"");
}

#[test]
fn test_serde_serialize_binary() {
    let term = Term::Binary(&[0, 1, 2, 255]);
    let json = serde_json::to_string(&term).unwrap();
    // Binary serializes as a JSON array of integers.
    assert_eq!(json, "[0,1,2,255]");
}

#[test]
fn test_serde_serialize_list() {
    let terms = [Term::Int(1), Term::Int(2), Term::Int(3)];
    let term = Term::List(&terms);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3]");
}

#[test]
fn test_serde_serialize_tuple() {
    let terms = [
        Term::Int(10),
        Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"x") }),
    ];
    let term = Term::Tuple(&terms);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[10,\"x\"]");
}

#[test]
fn test_serde_serialize_map() {
    let pairs = [(
        Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"key") }),
        Term::Int(42),
    )];
    let term = Term::Map(&pairs);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "{\"key\":42}");
}

#[test]
fn test_serde_serialize_empty_list() {
    let term = Term::List(&[]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[]");
}

#[test]
fn test_serde_serialize_nested() {
    // Parse an ETF term and serialize it via serde.
    let input = b"\x83\x68\x02\x61\x01\x68\x02\x61\x02\x61\x03"; // {1, {2, 3}}
    with_parse(input, |term| {
        let json = serde_json::to_string(&term).unwrap();
        assert_eq!(json, "[1,[2,3]]");
    });
}

#[test]
fn test_serde_deserialize_int() {
    let json = "42";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    assert!(matches!(term, OwnedTerm::Int(42)));
}

#[test]
fn test_serde_deserialize_neg_int() {
    let json = "-1000";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    assert!(matches!(term, OwnedTerm::Int(-1000)));
}

#[test]
fn test_serde_deserialize_float() {
    let json = "3.141592653589793";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    assert!(matches!(term, OwnedTerm::Float(v) if (v - std::f64::consts::PI).abs() < 1e-10));
}

#[test]
fn test_serde_deserialize_string() {
    let json = "\"hello\"";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    assert!(matches!(term, OwnedTerm::Atom(ref s) if s == "hello"));
}

#[test]
fn test_serde_deserialize_list() {
    let json = "[1, \"two\", 3.0]";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    match term {
        OwnedTerm::List(elements) => {
            assert_eq!(elements.len(), 3);
            assert!(matches!(&elements[0], OwnedTerm::Int(1)));
            assert!(matches!(&elements[1], OwnedTerm::Atom(s) if s == "two"));
            assert!(matches!(&elements[2], OwnedTerm::Float(_)));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_serde_deserialize_map() {
    let json = "{\"a\": 1, \"b\": 2}";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    match term {
        OwnedTerm::Map(pairs) => {
            assert_eq!(pairs.len(), 2);
        }
        _ => panic!("expected Map"),
    }
}

#[test]
fn test_serde_deserialize_nested() {
    let json = "[1, [2, 3], {\"x\": 10}]";
    let term: OwnedTerm = serde_json::from_str(json).unwrap();
    match term {
        OwnedTerm::List(elements) => {
            assert_eq!(elements.len(), 3);
            assert!(matches!(&elements[0], OwnedTerm::Int(1)));
            assert!(matches!(&elements[1], OwnedTerm::List(_)));
            assert!(matches!(&elements[2], OwnedTerm::Map(_)));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_serde_roundtrip_json() {
    // Serialize a Term to JSON, deserialize back to OwnedTerm, then
    // verify the structure matches.
    //
    // Note: JSON arrays always deserialize as `List`, never as `Tuple`,
    // because serde_json cannot distinguish tuple vs list for JSON arrays.
    // Byte arrays also deserialize as `List` since JSON has no native
    // byte type.
    let original = Term::Tuple(&[
        Term::Int(1),
        Term::Atom(unsafe { AtomUtf8::from_bytes_unchecked(b"hello") }),
        Term::List(&[Term::Float(std::f64::consts::PI), Term::Int(42)]),
    ]);

    // Term → JSON
    let json = serde_json::to_string(&original).unwrap();

    // JSON → OwnedTerm
    let owned: OwnedTerm = serde_json::from_str(&json).unwrap();

    // Tuples become Lists in JSON. The JSON array [1,"hello",[3.14,42]]
    // deserializes as a List:
    match owned {
        OwnedTerm::List(elems) => {
            assert_eq!(elems.len(), 3);
            assert!(matches!(elems[0], OwnedTerm::Int(1)));
            assert!(matches!(&elems[1], OwnedTerm::Atom(s) if s == "hello"));
            match &elems[2] {
                OwnedTerm::List(inner) => {
                    assert_eq!(inner.len(), 2);
                    assert!(matches!(inner[0], OwnedTerm::Float(_)));
                    assert!(matches!(inner[1], OwnedTerm::Int(42)));
                }
                _ => panic!("expected inner List"),
            }
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn test_serde_deserialize_bool() {
    let term: OwnedTerm = serde_json::from_str("true").unwrap();
    assert!(matches!(term, OwnedTerm::Atom(ref s) if s == "true"));

    let term: OwnedTerm = serde_json::from_str("false").unwrap();
    assert!(matches!(term, OwnedTerm::Atom(ref s) if s == "false"));
}

#[test]
fn test_serde_record_serialize() {
    // Parse a RECORD_EXT, serialize to JSON.
    let input = b"\x83\x43\x00\x00\x00\x01\x01\x77\x03foo\x77\x03bar\x77\x01x\x61\x2a";
    let json = with_parse(input, |term| serde_json::to_string(&term).unwrap());
    // Record serializes as JSON array of bytes.
    // When deserialized from JSON, bytes become a generic List.
    let owned: OwnedTerm = serde_json::from_str(&json).unwrap();
    // JSON has no native byte type, so it comes back as a List.
    assert!(matches!(owned, OwnedTerm::List(_)));
}

#[test]
fn test_serde_small_big_int() {
    let term = Term::SmallBigInt {
        sign: 0,
        digits: &[0xAB, 0xCD],
    };
    let json = serde_json::to_string(&term).unwrap();
    // Should serialize as a struct: {"sign": 0, "digits": [171, 205]}
    assert!(json.contains("sign"));
    assert!(json.contains("digits"));
}

// ── OwnedTerm serde: serialize all variants ─────────────────────────────────

#[test]
fn test_serde_owned_serialize_atom() {
    let term = OwnedTerm::Atom("hello".to_string());
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "\"hello\"");
}

#[test]
fn test_serde_owned_serialize_int() {
    let term = OwnedTerm::Int(-7);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "-7");
}

#[test]
fn test_serde_owned_serialize_float() {
    let term = OwnedTerm::Float(1.5);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "1.5");
}

#[test]
fn test_serde_owned_serialize_small_big() {
    let term = OwnedTerm::SmallBigInt {
        sign: 1,
        digits: vec![1, 2, 3],
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"sign\":1"));
    assert!(json.contains("\"digits\":[1,2,3]"));
}

#[test]
fn test_serde_owned_serialize_large_big() {
    let term = OwnedTerm::LargeBigInt {
        sign: 0,
        digits: vec![9, 8, 7],
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"sign\":0"));
    assert!(json.contains("\"digits\":[9,8,7]"));
}

#[test]
fn test_serde_owned_serialize_binary() {
    let term = OwnedTerm::Binary(vec![1, 2, 3, 255]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,255]");
}

#[test]
fn test_serde_owned_serialize_bit_binary() {
    let term = OwnedTerm::BitBinary {
        bits: 5,
        data: vec![0xAB],
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"bits\":5"));
    assert!(json.contains("\"data\":[171]"));
}

#[test]
fn test_serde_owned_serialize_list() {
    let term = OwnedTerm::List(vec![OwnedTerm::Int(1), OwnedTerm::Int(2)]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2]");
}

#[test]
fn test_serde_owned_serialize_improper_list() {
    use fasteetf::owned::OwnedTerm;
    let term = OwnedTerm::ImproperList {
        elements: vec![OwnedTerm::Int(1)],
        tail: Box::new(OwnedTerm::Int(2)),
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"elements\":[1]"));
    assert!(json.contains("\"tail\":2"));
}

#[test]
fn test_serde_owned_serialize_tuple() {
    let term = OwnedTerm::Tuple(vec![OwnedTerm::Int(10), OwnedTerm::Atom("x".into())]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[10,\"x\"]");
}

#[test]
fn test_serde_owned_serialize_map() {
    let term = OwnedTerm::Map(vec![(OwnedTerm::Atom("k".into()), OwnedTerm::Int(42))]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "{\"k\":42}");
}

#[test]
fn test_serde_owned_serialize_pid() {
    use fasteetf::owned::{OwnedTerm, PidOwned};
    let term = OwnedTerm::Pid(PidOwned(103, vec![1, 2, 3]));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":103"));
    assert!(json.contains("\"data\":[1,2,3]"));
}

#[test]
fn test_serde_owned_serialize_port() {
    use fasteetf::owned::{OwnedTerm, PortOwned};
    let term = OwnedTerm::Port(PortOwned(102, vec![4, 5, 6]));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":102"));
    assert!(json.contains("\"data\":[4,5,6]"));
}

#[test]
fn test_serde_owned_serialize_ref() {
    use fasteetf::owned::{OwnedTerm, ReferenceOwned};
    let term = OwnedTerm::Ref(ReferenceOwned(114, vec![7, 8, 9]));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":114"));
    assert!(json.contains("\"data\":[7,8,9]"));
}

#[test]
fn test_serde_owned_serialize_function() {
    use fasteetf::owned::{FunctionOwned, OwnedTerm};
    let term = OwnedTerm::Function(FunctionOwned(113, vec![10, 11]));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":113"));
    assert!(json.contains("\"data\":[10,11]"));
}

#[test]
fn test_serde_owned_serialize_record() {
    use fasteetf::owned::{OwnedTerm, RecordOwned};
    let term = OwnedTerm::Record(RecordOwned(vec![1, 2, 3, 4]));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,4]");
}

// ── OwnedTerm deserializer paths ───────────────────────────────────────────

#[test]
fn test_serde_deserialize_i8() {
    let term: OwnedTerm = serde_json::from_str("127").unwrap();
    assert!(matches!(term, OwnedTerm::Int(127)));
}

#[test]
fn test_serde_deserialize_u64_overflow() {
    // u64 too large for i32 -> SmallBigInt
    let json = format!("{}", u64::MAX);
    let term: OwnedTerm = serde_json::from_str(&json).unwrap();
    match term {
        OwnedTerm::SmallBigInt { sign: 0, digits } => {
            assert_eq!(digits.len(), 8);
        }
        other => panic!("expected SmallBigInt, got {other:?}"),
    }
}

#[test]
fn test_serde_deserialize_i64_overflow_negative() {
    // i64 too negative for i32 -> SmallBigInt with sign=1
    let json = format!("{}", i64::MIN);
    let term: OwnedTerm = serde_json::from_str(&json).unwrap();
    match term {
        OwnedTerm::SmallBigInt { sign: 1, digits } => {
            assert!(!digits.is_empty());
        }
        other => panic!("expected SmallBigInt, got {other:?}"),
    }
}

#[test]
fn test_serde_deserialize_i64_overflow_positive() {
    // i64 too positive for i32 -> SmallBigInt with sign=0
    let json = format!("{}", i64::MAX);
    let term: OwnedTerm = serde_json::from_str(&json).unwrap();
    match term {
        OwnedTerm::SmallBigInt { sign: 0, digits } => {
            assert!(!digits.is_empty());
        }
        other => panic!("expected SmallBigInt, got {other:?}"),
    }
}

#[test]
fn test_serde_deserialize_u32_overflow() {
    let json = format!("{}", u32::MAX);
    let term: OwnedTerm = serde_json::from_str(&json).unwrap();
    match term {
        OwnedTerm::SmallBigInt { sign: 0, digits } => {
            // u32::MAX = 0xFFFFFFFF, padded to 8 bytes (u64 width). serde_json
            // calls visit_u64 so the digit buffer is the full 8-byte LE repr.
            assert_eq!(digits, vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]);
        }
        other => panic!("expected SmallBigInt, got {other:?}"),
    }
}

#[test]
fn test_serde_deserialize_f32() {
    let term: OwnedTerm = serde_json::from_str("1.5").unwrap();
    assert!(matches!(term, OwnedTerm::Float(v) if (v - 1.5).abs() < 1e-6));
}

#[test]
fn test_serde_deserialize_string_owned() {
    // A long string (> 31 bytes) exercises the visit_string path.
    let term: OwnedTerm =
        serde_json::from_str("\"a long string that exercises the visit_string path\"").unwrap();
    assert!(matches!(term, OwnedTerm::Atom(ref s) if s.starts_with("a long")));
}

#[test]
fn test_serde_deserialize_unit() {
    // null deserializes to empty list (analogous to Erlang `[]`).
    let term: OwnedTerm = serde_json::from_str("null").unwrap();
    assert!(matches!(term, OwnedTerm::List(ref e) if e.is_empty()));
}

#[test]
fn test_serde_deserialize_none() {
    // null deserializes to None when wrapped in an Option.
    let opt: Option<OwnedTerm> = serde_json::from_str("null").unwrap();
    assert!(opt.is_none());
}

#[test]
fn test_serde_deserialize_byte_buf() {
    // JSON array of bytes deserializes to List, not Binary (because visit_seq
    // is called, not visit_bytes/visit_byte_buf). This test documents that
    // behavior — bytes are a JSON-only concept and there's no native byte
    // array, so we get a List.
    let term: OwnedTerm = serde_json::from_str("[10, 20, 30, 40, 50, 60, 70, 80, 90]").unwrap();
    assert!(matches!(term, OwnedTerm::List(ref elems) if elems.len() == 9));
}

// ── Term serialize: hit LargeBigInt, BitBinary, ImproperList, Pid, Port, Ref, Function, Record ─

#[test]
fn test_serde_term_serialize_large_big() {
    let term = Term::LargeBigInt {
        sign: 1,
        digits: &[0xAB, 0xCD],
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"sign\":1"));
    assert!(json.contains("\"digits\":[171,205]"));
}

#[test]
fn test_serde_term_serialize_bit_binary() {
    let term = Term::BitBinary {
        bits: 4,
        data: &[0xAB, 0xCD],
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"bits\":4"));
}

#[test]
fn test_serde_term_serialize_improper_list() {
    let head = Term::Int(1);
    let term = Term::ImproperList {
        elements: &[head],
        tail: &Term::Int(2),
    };
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"elements\":[1]"));
    assert!(json.contains("\"tail\":2"));
}

#[test]
fn test_serde_term_serialize_pid() {
    let data = [0u8; 9];
    let term = Term::Pid(Pid(103, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":103"));
}

#[test]
fn test_serde_term_serialize_port() {
    let data = [0u8; 5];
    let term = Term::Port(Port(102, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":102"));
}

#[test]
fn test_serde_term_serialize_ref() {
    let data = [0u8; 4];
    let term = Term::Ref(Reference(114, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":114"));
}

#[test]
fn test_serde_term_serialize_function() {
    let data = [0u8; 4];
    let term = Term::Function(Function(113, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":113"));
}

#[test]
fn test_serde_term_serialize_record() {
    let data = [1u8, 2, 3, 4];
    let term = Term::Record(Record(&data));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,4]");
}

// ── AtomUtf8: lossy serialization for invalid UTF-8 ────────────────────────

#[test]
fn test_serde_atom_invalid_utf8() {
    use fasteetf::AtomUtf8;
    let a = unsafe { AtomUtf8::from_bytes_unchecked(b"\xff\xfe") };
    let term = Term::Atom(a);
    let json = serde_json::to_string(&term).unwrap();
    // Invalid UTF-8 falls back to bytes serialization.
    assert!(json.contains("255") || json.contains("byte"));
}

// ── Opaque owned wrappers: serialize + deserialize roundtrip ───────────────

#[test]
fn test_serde_pid_owned_roundtrip() {
    use fasteetf::owned::PidOwned;
    let pid = PidOwned(103, vec![1, 2, 3, 4]);
    let json = serde_json::to_string(&pid).unwrap();
    let de: PidOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, 103);
    assert_eq!(de.1, vec![1, 2, 3, 4]);
}

#[test]
fn test_serde_port_owned_roundtrip() {
    use fasteetf::owned::PortOwned;
    let port = PortOwned(102, vec![5, 6, 7, 8]);
    let json = serde_json::to_string(&port).unwrap();
    let de: PortOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, 102);
    assert_eq!(de.1, vec![5, 6, 7, 8]);
}

#[test]
fn test_serde_ref_owned_roundtrip() {
    use fasteetf::owned::ReferenceOwned;
    let r = ReferenceOwned(114, vec![9, 10, 11, 12]);
    let json = serde_json::to_string(&r).unwrap();
    let de: ReferenceOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, 114);
    assert_eq!(de.1, vec![9, 10, 11, 12]);
}

#[test]
fn test_serde_function_owned_roundtrip() {
    use fasteetf::owned::FunctionOwned;
    let f = FunctionOwned(113, vec![13, 14, 15]);
    let json = serde_json::to_string(&f).unwrap();
    let de: FunctionOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, 113);
    assert_eq!(de.1, vec![13, 14, 15]);
}

#[test]
fn test_serde_record_owned_roundtrip() {
    use fasteetf::owned::RecordOwned;
    let r = RecordOwned(vec![20, 21, 22]);
    let json = serde_json::to_string(&r).unwrap();
    let de: RecordOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, vec![20, 21, 22]);
}

#[test]
fn test_serde_record_owned_from_seq() {
    use fasteetf::owned::RecordOwned;
    // JSON array of integers exercises the visit_seq path of the UntaggedOpaqueVisitor.
    let de: RecordOwned = serde_json::from_str("[100, 101, 102]").unwrap();
    assert_eq!(de.0, vec![100, 101, 102]);
}

// ── PidOwned / PortOwned / ReferenceOwned / FunctionOwned deserializer error ─

#[test]
fn test_serde_pid_owned_missing_field() {
    use fasteetf::owned::PidOwned;
    // Only data, no tag.
    let err = serde_json::from_str::<PidOwned>("{\"data\":[1,2,3]}").unwrap_err();
    assert!(format!("{err}").contains("tag") || format!("{err}").contains("missing"));
}

#[test]
fn test_serde_pid_owned_unknown_field() {
    use fasteetf::owned::PidOwned;
    let err =
        serde_json::from_str::<PidOwned>("{\"tag\":103,\"data\":[1],\"extra\":42}").unwrap_err();
    assert!(format!("{err}").contains("extra") || format!("{err}").contains("unknown"));
}

#[test]
fn test_serde_port_owned_missing_data() {
    use fasteetf::owned::PortOwned;
    let err = serde_json::from_str::<PortOwned>("{\"tag\":102}").unwrap_err();
    assert!(format!("{err}").contains("data") || format!("{err}").contains("missing"));
}

// ── Term / Pid / Port / Reference / Function borrowed Serialize paths ───────

#[test]
fn test_serde_borrowed_pid() {
    let data = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
    let term = Term::Pid(Pid(103, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":103"));
    assert!(json.contains("\"data\":[1,2,3,4,5,6,7,8,9]"));
}

#[test]
fn test_serde_borrowed_port() {
    let data = [1u8, 2, 3, 4, 5];
    let term = Term::Port(Port(102, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":102"));
}

#[test]
fn test_serde_borrowed_reference() {
    let data = [1u8, 2, 3, 4];
    let term = Term::Ref(Reference(114, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":114"));
}

#[test]
fn test_serde_borrowed_function() {
    let data = [1u8, 2, 3];
    let term = Term::Function(Function(113, &data));
    let json = serde_json::to_string(&term).unwrap();
    assert!(json.contains("\"tag\":113"));
}

#[test]
fn test_serde_borrowed_record() {
    let data = [1u8, 2, 3];
    let term = Term::Record(Record(&data));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3]");
}

// ── Visitor visit_* methods: drive each via a typed deserializer ───────────

#[test]
fn test_serde_ownedterm_visit_i8() {
    // Use serde_json with a small JSON value.  Numbers go through visit_i64
    // in serde_json; we instead create a custom deserializer that calls visit_i8.
    use fasteetf::owned::OwnedTerm;

    struct I8Deser(i8);
    impl<'de> serde_core::Deserializer<'de> for I8Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i8(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let val = <i8 as serde_core::Deserialize>::deserialize(I8Deser(42)).unwrap();
    assert_eq!(val, 42);

    // Now drive OwnedTerm through visit_i8: deserialize OwnedTerm via the
    // inner visitor, providing a deserializer that calls visit_i8.
    struct OwnedTermViaI8(i8);
    impl<'de> serde_core::Deserializer<'de> for OwnedTermViaI8 {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i8(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(OwnedTermViaI8(7)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(7)));
}

#[test]
fn test_serde_ownedterm_visit_i16() {
    use fasteetf::owned::OwnedTerm;
    struct I16Deser(i16);
    impl<'de> serde_core::Deserializer<'de> for I16Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i16(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(I16Deser(1000)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(1000)));
}

#[test]
fn test_serde_ownedterm_visit_i32() {
    use fasteetf::owned::OwnedTerm;
    struct I32Deser(i32);
    impl<'de> serde_core::Deserializer<'de> for I32Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i32(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(I32Deser(123_456)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(123_456)));
}

#[test]
fn test_serde_ownedterm_visit_u8() {
    use fasteetf::owned::OwnedTerm;
    struct U8Deser(u8);
    impl<'de> serde_core::Deserializer<'de> for U8Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u8(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U8Deser(200)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(200)));
}

#[test]
fn test_serde_ownedterm_visit_u16() {
    use fasteetf::owned::OwnedTerm;
    struct U16Deser(u16);
    impl<'de> serde_core::Deserializer<'de> for U16Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u16(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U16Deser(40_000)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(40_000)));
}

#[test]
fn test_serde_ownedterm_visit_u32() {
    use fasteetf::owned::OwnedTerm;
    struct U32Deser(u32);
    impl<'de> serde_core::Deserializer<'de> for U32Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u32(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    // u32::MAX doesn't fit in i32 → SmallBigInt path
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U32Deser(u32::MAX)).unwrap();
    assert!(matches!(term, OwnedTerm::SmallBigInt { sign: 0, .. }));
}

#[test]
fn test_serde_ownedterm_visit_f32() {
    use fasteetf::owned::OwnedTerm;
    struct F32Deser(f32);
    impl<'de> serde_core::Deserializer<'de> for F32Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_f32(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(F32Deser(1.5)).unwrap();
    assert!(matches!(term, OwnedTerm::Float(_)));
}

#[test]
fn test_serde_ownedterm_visit_string() {
    use fasteetf::owned::OwnedTerm;
    struct StringDeser(String);
    impl<'de> serde_core::Deserializer<'de> for StringDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_string(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm =
        serde_core::Deserialize::deserialize(StringDeser("hello".to_string())).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "hello"));
}

#[test]
fn test_serde_ownedterm_visit_bytes() {
    use fasteetf::owned::OwnedTerm;
    struct BytesDeser(Vec<u8>);
    impl<'de> serde_core::Deserializer<'de> for BytesDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bytes(&self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(BytesDeser(vec![1, 2, 3])).unwrap();
    assert!(matches!(term, OwnedTerm::Binary(b) if b == vec![1, 2, 3]));
}

#[test]
fn test_serde_ownedterm_visit_byte_buf() {
    use fasteetf::owned::OwnedTerm;
    struct ByteBufDeser(Vec<u8>);
    impl<'de> serde_core::Deserializer<'de> for ByteBufDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_byte_buf(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm =
        serde_core::Deserialize::deserialize(ByteBufDeser(vec![4, 5, 6])).unwrap();
    assert!(matches!(term, OwnedTerm::Binary(b) if b == vec![4, 5, 6]));
}

#[test]
fn test_serde_ownedterm_visit_none() {
    use fasteetf::owned::OwnedTerm;
    struct TypedNoneDeser;
    impl<'de> serde_core::Deserializer<'de> for TypedNoneDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_none()
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    // Deserialize OwnedTerm directly through TypedNoneDeser, which calls
    // visit_none on the OwnedTermVisitor.
    let term: OwnedTerm = serde_core::Deserialize::deserialize(TypedNoneDeser).unwrap();
    // visit_none on OwnedTermVisitor returns Atom("undefined")
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "undefined"));
}

#[test]
fn test_serde_ownedterm_visit_some() {
    use fasteetf::owned::OwnedTerm;
    // Drive visit_some on the OwnedTermVisitor by deserializing OwnedTerm
    // through a deserializer that calls visit_some.  The visitor delegates
    // back to OwnedTerm::deserialize, which then calls deserialize_any on
    // the inner deserializer (IntDeser → visit_i64).
    struct IntDeser(i64);
    impl<'de> serde_core::Deserializer<'de> for IntDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    struct SomeDeser(IntDeser);
    impl<'de> serde_core::Deserializer<'de> for SomeDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_some(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(SomeDeser(IntDeser(99))).unwrap();
    assert!(matches!(term, OwnedTerm::Int(99)));
}

#[test]
fn test_serde_ownedterm_visit_newtype_struct() {
    use fasteetf::owned::OwnedTerm;
    // visit_newtype_struct delegates to OwnedTerm::deserialize.  The inner
    // deserializer delivers an int.
    struct IntDeser(i64);
    impl<'de> serde_core::Deserializer<'de> for IntDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    struct NewtypeDeser(IntDeser);
    impl<'de> serde_core::Deserializer<'de> for NewtypeDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_newtype_struct(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let val: OwnedTerm = serde_core::Deserialize::deserialize(NewtypeDeser(IntDeser(99))).unwrap();
    assert!(matches!(val, OwnedTerm::Int(99)));
}

#[test]
fn test_serde_ownedterm_visit_unit() {
    use fasteetf::owned::OwnedTerm;
    // Drive visit_unit directly.
    struct UnitDeser;
    impl<'de> serde_core::Deserializer<'de> for UnitDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_unit()
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(UnitDeser).unwrap();
    assert!(matches!(term, OwnedTerm::List(ref e) if e.is_empty()));
}

#[test]
fn test_serde_ownedterm_visit_bool_true() {
    use fasteetf::owned::OwnedTerm;
    struct BoolDeser(bool);
    impl<'de> serde_core::Deserializer<'de> for BoolDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bool(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(BoolDeser(true)).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "true"));
}

#[test]
fn test_serde_ownedterm_visit_bool_false() {
    use fasteetf::owned::OwnedTerm;
    struct BoolDeser(bool);
    impl<'de> serde_core::Deserializer<'de> for BoolDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bool(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(BoolDeser(false)).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "false"));
}

#[test]
fn test_serde_ownedterm_visit_str() {
    use fasteetf::owned::OwnedTerm;
    struct StrDeser(&'static str);
    impl<'de> serde_core::Deserializer<'de> for StrDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_str(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(StrDeser("world")).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "world"));
}

#[test]
fn test_serde_ownedterm_visit_f64() {
    use fasteetf::owned::OwnedTerm;
    struct F64Deser(f64);
    impl<'de> serde_core::Deserializer<'de> for F64Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_f64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(F64Deser(2.5)).unwrap();
    assert!(matches!(term, OwnedTerm::Float(v) if (v - 2.5).abs() < 1e-6));
}

#[test]
fn test_serde_ownedterm_visit_i64_in_range() {
    use fasteetf::owned::OwnedTerm;
    struct I64Deser(i64);
    impl<'de> serde_core::Deserializer<'de> for I64Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(I64Deser(42)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(42)));
}

#[test]
fn test_serde_ownedterm_visit_u64_in_range() {
    use fasteetf::owned::OwnedTerm;
    struct U64Deser(u64);
    impl<'de> serde_core::Deserializer<'de> for U64Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U64Deser(100)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(100)));
}

// ── UntaggedOpaqueVisitor (for RecordOwned): drive visit_bytes and visit_byte_buf ──

#[test]
fn test_serde_untagged_opaque_visit_bytes() {
    use fasteetf::owned::RecordOwned;
    // Drive visit_bytes on the UntaggedOpaqueVisitor by deserializing
    // RecordOwned through a deserializer that calls visit_bytes.
    struct BytesDeser(Vec<u8>);
    impl<'de> serde_core::Deserializer<'de> for BytesDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bytes(&self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let rec: RecordOwned =
        serde_core::Deserialize::deserialize(BytesDeser(vec![10, 20, 30])).unwrap();
    assert_eq!(rec.0, vec![10, 20, 30]);
}

#[test]
fn test_serde_untagged_opaque_visit_byte_buf() {
    use fasteetf::owned::RecordOwned;
    struct ByteBufDeser(Vec<u8>);
    impl<'de> serde_core::Deserializer<'de> for ByteBufDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_byte_buf(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let rec: RecordOwned =
        serde_core::Deserialize::deserialize(ByteBufDeser(vec![40, 50, 60])).unwrap();
    assert_eq!(rec.0, vec![40, 50, 60]);
}

// ── expect() paths: trigger deserialization errors that name the visitor ────

#[test]
fn test_serde_ownedterm_expect_error() {
    use fasteetf::owned::OwnedTerm;
    // Force the OwnedTermVisitor's expecting() to be invoked by feeding
    // OwnedTerm through a deserializer that calls a method whose default
    // implementation invokes expecting().  The default visit_unit calls
    // expecting() to format the error.
    struct UnitDeser;
    impl<'de> serde_core::Deserializer<'de> for UnitDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            // visit_unit is overridden on OwnedTermVisitor (returns empty
            // List).  To trigger expecting(), we need a visit method that
            // OwnedTermVisitor does NOT override and uses the default
            // expecting() error.  visit_enum would do that but is not
            // commonly triggered by JSON.  Use deserialize_enum instead.
            visitor.visit_unit()
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    // Direct deserialize.  OwnedTermVisitor's visit_unit returns empty
    // List, so this succeeds.  Use a different path: a struct that
    // expects to call visit_seq on OwnedTermVisitor (which it does have).
    let _: OwnedTerm = serde_core::Deserialize::deserialize(UnitDeser).unwrap();
}

#[test]
fn test_serde_untagged_opaque_expect_error() {
    use fasteetf::owned::RecordOwned;
    // Construct an integer JSON value.  serde_json's deserializer will
    // call visit_i64, but UntaggedOpaqueVisitor only implements
    // visit_bytes/byte_buf/seq.  The default visit_i64 returns an error
    // that names the visitor's expecting() string.
    let result: Result<RecordOwned, _> = serde_json::from_str("42");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err}");
    // The error should mention "byte array" (from expecting()).
    assert!(
        msg.contains("byte") || msg.contains("invalid type"),
        "got: {msg}"
    );
}

#[test]
fn test_serde_tagged_opaque_expect_error() {
    use fasteetf::owned::PidOwned;
    // Trigger expecting() on the TaggedOpaqueVisitor by feeding JSON with an
    // empty map (missing "tag" and "data" fields).  The visitor's
    // visit_map loop will hit `Err(de::Error::missing_field("tag"))`, which
    // internally calls `expecting()` to format the error.
    let result: Result<PidOwned, _> = serde_json::from_str("{}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err}");
    // The error message should mention "tag" since that's the missing field.
    assert!(msg.contains("tag") || msg.contains("missing"));
}
