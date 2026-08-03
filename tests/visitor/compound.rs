use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

// ── Tuples ──────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_tuple() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x68\x02\x61\x01\x61\x02", &mut v).unwrap();
    assert_eq!(
        v.events,
        vec!["tuple_start(arity=2)", "int(1)", "int(2)", "tuple_end"]
    );
}

#[test]
fn test_visitor_large_tuple() {
    // LARGE_TUPLE_EXT (105): arity encoded as 4-byte big-endian.
    let mut v = EventLog::default();
    run_visitor(&[131, 105, 0, 0, 0, 2, 97, 1, 97, 2], &mut v).unwrap();
    assert_eq!(
        v.events,
        vec!["tuple_start(arity=2)", "int(1)", "int(2)", "tuple_end"]
    );
}

// ── Lists ───────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_nil_list_balanced() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x6a", &mut v).unwrap();
    assert_eq!(v.events, vec!["list_start(len=0)", "list_end"]);
}

#[test]
fn test_visitor_proper_list_balanced() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x6c\x00\x00\x00\x02\x61\x01\x61\x02\x6a", &mut v).unwrap();
    assert_eq!(
        v.events,
        vec!["list_start(len=2)", "int(1)", "int(2)", "list_end"]
    );
}

#[test]
fn test_visitor_improper_list() {
    // [1 | 2] -> LIST_EXT len=1, int(1), int(2) as tail (not nil)
    let mut v = EventLog::default();
    run_visitor(&[131, 108, 0, 0, 0, 1, 97, 1, 97, 2], &mut v).unwrap();
    assert_eq!(
        v.events,
        vec![
            "list_start(len=1)",
            "int(1)",
            "improper_list_tail",
            "int(2)",
            "improper_list_end",
        ]
    );
}

#[test]
fn test_visitor_nested() {
    let mut v = EventLog::default();
    // tuple(tuple(1))
    let input = b"\x83\x68\x01\x68\x01\x61\x01";
    run_visitor(input, &mut v).unwrap();
    assert_eq!(
        v.events,
        vec![
            "tuple_start(arity=1)",
            "tuple_start(arity=1)",
            "int(1)",
            "tuple_end",
            "tuple_end",
        ]
    );
}

// ── String (STRING_EXT) ─────────────────────────────────────────────────────

#[test]
fn test_visitor_string() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x6b\x00\x04abcd", &mut v).unwrap();
    // STRING_EXT is delivered as a single call to visit_string with the raw bytes.
    assert_eq!(v.events, vec!["string([97, 98, 99, 100])"]);
}

// ── Maps ────────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_map() {
    // MAP_EXT #{1 => 2} (small int keys/values)
    let mut v = EventLog::default();
    run_visitor(&[131, 116, 0, 0, 0, 1, 97, 1, 97, 2], &mut v).unwrap();
    assert_eq!(
        v.events,
        vec!["map_start(arity=1)", "int(1)", "int(2)", "map_end"]
    );
}

// ── Binaries ────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_binary() {
    let mut v = EventLog::default();
    run_visitor(&[131, 109, 0, 0, 0, 4, b'a', b'b', b'c', b'd'], &mut v).unwrap();
    assert_eq!(v.events, vec!["binary([97, 98, 99, 100])"]);
}

#[test]
fn test_visitor_bit_binary() {
    let mut v = EventLog::default();
    run_visitor(&[131, 77, 0, 0, 0, 1, 3, 0b1010_0000], &mut v).unwrap();
    assert_eq!(v.events, vec!["bit_binary(bits=3,data=[160])"]);
}
