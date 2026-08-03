use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

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
            assert_eq!(digits, vec![0xFF, 0xFF, 0xFF, 0xFF]);
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

// ── PidOwned / PortOwned / ReferenceOwned / FunctionOwned deserializer error ─

#[test]
fn test_serde_pid_owned_missing_field() {
    use fasteetf::owned::PidOwned;
    let err = serde_json::from_str::<PidOwned>("{\"data\":[1,2,3]}").unwrap_err();
    assert!(format!("{err}").contains("invalid"));
}

#[test]
fn test_serde_pid_owned_unknown_field() {
    use fasteetf::owned::PidOwned;
    let err = serde_json::from_str::<PidOwned>("{\"extra\":42}").unwrap_err();
    assert!(format!("{err}").contains("invalid"));
}

#[test]
fn test_serde_port_owned_missing_data() {
    use fasteetf::owned::PortOwned;
    let err = serde_json::from_str::<PortOwned>("{\"tag\":102}").unwrap_err();
    assert!(format!("{err}").contains("invalid"));
}
