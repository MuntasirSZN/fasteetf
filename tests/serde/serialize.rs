use super::*;

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
    let term = Term::BigInt {
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
    let term = OwnedTerm::SmallBigInt {
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
    let term = OwnedTerm::Pid(PidOwned(vec![103, 1, 2, 3]));
    // PidOwned stores the raw bytes (including tag)
    // Just check that it serializes without error
    let bytes = serde_json::to_vec(&term).unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn test_serde_owned_serialize_port() {
    use fasteetf::owned::{OwnedTerm, PortOwned};
    let term = OwnedTerm::Port(PortOwned(vec![4, 5, 6]));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[4,5,6]");
}

#[test]
fn test_serde_owned_serialize_ref() {
    use fasteetf::owned::{OwnedTerm, ReferenceOwned};
    let term = OwnedTerm::Ref(ReferenceOwned(vec![7, 8, 9]));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[7,8,9]");
}

#[test]
fn test_serde_owned_serialize_function() {
    use fasteetf::owned::{FunctionOwned, OwnedTerm};
    let term = OwnedTerm::Function(FunctionOwned(vec![10, 11]));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[10,11]");
}

#[test]
fn test_serde_owned_serialize_record() {
    use fasteetf::owned::{OwnedTerm, RecordOwned};
    let term = OwnedTerm::Record(RecordOwned(vec![1, 2, 3, 4]));
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,4]");
}

// ── Term serialize: hit LargeBigInt, BitBinary, ImproperList, Pid, Port, Ref, Function, Record ─

#[test]
fn test_serde_term_serialize_large_big() {
    let term = Term::BigInt {
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
    let tail = Term::Int(2);
    let elements = &[head, tail];
    let term = Term::ImproperList(elements);
    // ImproperList serializes as a sequence
    let _json = serde_json::to_string(&term).unwrap();
    // Just check it serializes without error
}

#[test]
fn test_serde_term_serialize_pid() {
    let data = [103u8, 1, 2, 3, 4, 5, 6, 7, 8];
    let term = Term::Pid(&data);
    // Pid serializes as bytes
    let _json = serde_json::to_string(&term).unwrap();
    // Just check it serializes without error
}

#[test]
fn test_serde_term_serialize_port() {
    let data = [102u8, 1, 2, 3, 4];
    let term = Term::Port(&data);
    // Port serializes as bytes
    let _json = serde_json::to_string(&term).unwrap();
}

#[test]
fn test_serde_term_serialize_ref() {
    let term = Term::Ref(&[114u8, 1, 2, 3]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[114,1,2,3]");
}

#[test]
fn test_serde_term_serialize_function() {
    let term = Term::Function(&[113u8, 1, 2, 3]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[113,1,2,3]");
}

#[test]
fn test_serde_term_serialize_record() {
    let term = Term::Record(&[1, 2, 3, 4]);
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

// ── Term / Pid / Port / Reference / Function borrowed Serialize paths ───────

#[test]
fn test_serde_borrowed_pid() {
    let data = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
    let term = Term::Pid(&data);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,4,5,6,7,8,9]");
}

#[test]
fn test_serde_borrowed_port() {
    let data = [1u8, 2, 3, 4, 5];
    let term = Term::Port(&data);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,4,5]");
}

#[test]
fn test_serde_borrowed_reference() {
    let term = Term::Ref(&[114u8, 1, 2, 3]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[114,1,2,3]");
}

#[test]
fn test_serde_borrowed_function() {
    let term = Term::Function(&[113u8, 1, 2, 3]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[113,1,2,3]");
}

#[test]
fn test_serde_borrowed_record() {
    let term = Term::Record(&[1, 2, 3, 4]);
    let json = serde_json::to_string(&term).unwrap();
    assert_eq!(json, "[1,2,3,4]");
}
