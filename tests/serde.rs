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
