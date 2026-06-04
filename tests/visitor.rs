// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for the Visitor API.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

use fasteetf::*;

// A single catch-all visitor used by most tests.  Every visit method pushes a
// stringified event onto a `Vec`, so each test can assert on the exact sequence
// of events the parser emits.
#[derive(Default)]
struct EventLog {
    events: Vec<String>,
}

impl Visitor for EventLog {
    type Error = EtfError;

    fn visit_int(&mut self, value: i32) -> Result<(), Self::Error> {
        self.events.push(format!("int({value})"));
        Ok(())
    }

    fn visit_big_int(&mut self, sign: u8, digits: &[u8]) -> Result<(), Self::Error> {
        self.events
            .push(format!("big(sign={sign},digits={digits:?})"));
        Ok(())
    }

    fn visit_float(&mut self, value: f64) -> Result<(), Self::Error> {
        self.events.push(format!("float({value})"));
        Ok(())
    }

    fn visit_atom(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!(
            "atom({})",
            std::str::from_utf8(bytes).unwrap_or("<bad utf8>")
        ));
        Ok(())
    }

    fn visit_binary(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("binary({data:?})"));
        Ok(())
    }

    fn visit_bit_binary(&mut self, bits: u8, data: &[u8]) -> Result<(), Self::Error> {
        self.events
            .push(format!("bit_binary(bits={bits},data={data:?})"));
        Ok(())
    }

    fn visit_tuple_start(&mut self, arity: usize) -> Result<(), Self::Error> {
        self.events.push(format!("tuple_start(arity={arity})"));
        Ok(())
    }

    fn visit_tuple_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("tuple_end".to_string());
        Ok(())
    }

    fn visit_list_start(&mut self, len: usize) -> Result<(), Self::Error> {
        self.events.push(format!("list_start(len={len})"));
        Ok(())
    }

    fn visit_list_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("list_end".to_string());
        Ok(())
    }

    fn visit_improper_list_tail(&mut self) -> Result<(), Self::Error> {
        self.events.push("improper_list_tail".to_string());
        Ok(())
    }

    fn visit_improper_list_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("improper_list_end".to_string());
        Ok(())
    }

    fn visit_map_start(&mut self, arity: usize) -> Result<(), Self::Error> {
        self.events.push(format!("map_start(arity={arity})"));
        Ok(())
    }

    fn visit_map_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("map_end".to_string());
        Ok(())
    }

    fn visit_pid(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("pid({data:?})"));
        Ok(())
    }

    fn visit_port(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("port({data:?})"));
        Ok(())
    }

    fn visit_reference(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("ref({data:?})"));
        Ok(())
    }

    fn visit_function(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("fun({data:?})"));
        Ok(())
    }

    fn visit_record(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("record({data:?})"));
        Ok(())
    }

    fn visit_string(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("string({data:?})"));
        Ok(())
    }
}

fn run_visitor(input: &[u8], events: &mut EventLog) -> Result<(), EtfError> {
    parse_etf_with_visitor(input, None, None, events, &Limits::default())
}

// ── Scalars ─────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_small_int() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x61\x2a", &mut v).unwrap();
    assert_eq!(v.events, vec!["int(42)"]);
}

#[test]
fn test_visitor_integer_ext() {
    // 255 (small-positive bound)
    let mut v = EventLog::default();
    run_visitor(b"\x83\x62\x00\x00\x00\xff", &mut v).unwrap();
    assert_eq!(v.events, vec!["int(255)"]);

    // -1 (negative, two's complement)
    let mut v = EventLog::default();
    run_visitor(b"\x83\x62\xff\xff\xff\xff", &mut v).unwrap();
    assert_eq!(v.events, vec!["int(-1)"]);
}

#[test]
fn test_visitor_new_float() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x46\x40\x09\x21\xfb\x54\x44\x2d\x18", &mut v).unwrap();
    assert_eq!(v.events.len(), 1);
    assert!(v.events[0].starts_with("float("));
    assert!(v.events[0].contains("3.14"));
}

#[test]
fn test_visitor_legacy_float() {
    let content = format!("{:<30.20e}", 42.0f64);
    assert_eq!(content.len(), 30);
    let mut buf = vec![131, 99];
    buf.extend_from_slice(content.as_bytes());
    buf.push(0);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 1);
    assert!(v.events[0].starts_with("float(42"));
}

#[test]
fn test_visitor_small_big() {
    // 2-byte bignum, sign=0, digits=[0, 1]
    let mut v = EventLog::default();
    run_visitor(b"\x83\x6e\x02\x00\x00\x01", &mut v).unwrap();
    assert_eq!(v.events, vec!["big(sign=0,digits=[0, 1])"]);
}

#[test]
fn test_visitor_large_big() {
    let mut v = EventLog::default();
    run_visitor(&[131, 111, 0, 0, 0, 1, 0, 42], &mut v).unwrap();
    assert_eq!(v.events, vec!["big(sign=0,digits=[42])"]);
}

#[test]
fn test_visitor_small_atom() {
    let mut v = EventLog::default();
    run_visitor(b"\x83\x77\x03foo", &mut v).unwrap();
    assert_eq!(v.events, vec!["atom(foo)"]);
}

#[test]
fn test_visitor_utf8_atom() {
    // 300 bytes — uses ATOM_UTF8_EXT (118).
    let mut bytes = vec![131, 118];
    let name = "x".repeat(300);
    let name_bytes = name.as_bytes();
    bytes.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
    bytes.extend_from_slice(name_bytes);
    let mut v = EventLog::default();
    run_visitor(&bytes, &mut v).unwrap();
    assert_eq!(v.events.len(), 1);
    assert!(v.events[0].starts_with("atom(x"));
}

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

// ── PIDs ────────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_pid_legacy() {
    // PID_EXT (103) with node "node" (4 bytes), ID=1, Serial=1, Creation=1.
    // The visitor dispatches on the node atom first, then emits visit_pid.
    let mut buf = vec![131, 103];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]); // ID
    buf.extend_from_slice(&[0, 0, 0, 1]); // Serial
    buf.push(1); // Creation
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("pid("));
}

#[test]
fn test_visitor_pid_new() {
    // NEW_PID_EXT (88) with node "node" (4 bytes), ID=1, Serial=1, 4-byte Creation=1.
    let mut buf = vec![131, 88];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]); // ID
    buf.extend_from_slice(&[0, 0, 0, 1]); // Serial
    buf.extend_from_slice(&[0, 0, 0, 1]); // Creation (4 bytes)
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("pid("));
}

// ── Ports ───────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_port_legacy() {
    // PORT_EXT (102) with node "node", ID=1, 1-byte Creation=1.
    let mut buf = vec![131, 102];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.push(1);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("port("));
}

#[test]
fn test_visitor_port_new() {
    // NEW_PORT_EXT (89) with node "node", ID=1, 4-byte Creation=1.
    let mut buf = vec![131, 89];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("port("));
}

#[test]
fn test_visitor_port_v4() {
    // V4_PORT_EXT (120) with node "node", 8-byte ID=1, 4-byte Creation=1.
    let mut buf = vec![131, 120];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("port("));
}

// ── References ──────────────────────────────────────────────────────────────

#[test]
fn test_visitor_ref_legacy() {
    // NEW_REFERENCE_EXT (114): Len=1, node, 1-byte Creation, 1 ID word.
    // The visitor dispatches on the node atom first, then emits visit_reference.
    let mut buf = vec![131, 114, 0, 1];
    buf.extend_from_slice(b"\x77\x04node");
    buf.push(1);
    buf.extend_from_slice(&[0, 0, 0, 7]);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("ref("));
}

#[test]
fn test_visitor_ref_newer() {
    // NEWER_REFERENCE_EXT (90): Len=1, node, 4-byte Creation, 1 ID word.
    let mut buf = vec![131, 90, 0, 1];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 7]);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events.len(), 2);
    assert_eq!(v.events[0], "atom(node)");
    assert!(v.events[1].starts_with("ref("));
}

// ── Functions ───────────────────────────────────────────────────────────────

#[test]
fn test_visitor_new_fun() {
    // NEW_FUN_EXT: Size=8, then 4 bytes of payload (Size already consumed by parser).
    // Size includes the Size field itself (4) so the remaining payload is 4 bytes.
    let mut buf = vec![131, 112, 0, 0, 0, 8];
    buf.extend_from_slice(&[1, 2, 3, 4]);
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    assert_eq!(v.events, vec!["fun([1, 2, 3, 4])"]);
}

#[test]
fn test_visitor_export() {
    // EXPORT_EXT: Module, Function, Arity (each encoded as a term).
    // The visitor dispatches on each sub-term before emitting visit_function.
    let mut buf = vec![131, 113];
    buf.extend_from_slice(b"\x77\x05lists"); // atom "lists"
    buf.extend_from_slice(b"\x77\x03map"); // atom "map"
    buf.extend_from_slice(b"\x61\x02"); // small int 2
    let mut v = EventLog::default();
    run_visitor(&buf, &mut v).unwrap();
    // Expected: atom(lists), atom(map), int(2), then the wrapping fun.
    assert_eq!(v.events.len(), 4);
    assert_eq!(v.events[0], "atom(lists)");
    assert_eq!(v.events[1], "atom(map)");
    assert_eq!(v.events[2], "int(2)");
    assert!(v.events[3].starts_with("fun("));
}

// ── Records ─────────────────────────────────────────────────────────────────

#[test]
fn test_visitor_record() {
    // RECORD_EXT: #Fields=1, Flags=1, Module=foo, Name=bar, FieldName=[x], Values=[42]
    // The visitor dispatches on the Module and Name atoms, the FieldName
    // atom, and the Value (an int), then emits visit_record.
    let input = b"\x83\x43\x00\x00\x00\x01\x01\x77\x03foo\x77\x03bar\x77\x01x\x61\x2a";
    let mut v = EventLog::default();
    run_visitor(input, &mut v).unwrap();
    // atom(foo), atom(bar), atom(x), int(42), then the wrapping record.
    assert_eq!(v.events.len(), 5);
    assert_eq!(v.events[0], "atom(foo)");
    assert_eq!(v.events[1], "atom(bar)");
    assert_eq!(v.events[2], "atom(x)");
    assert_eq!(v.events[3], "int(42)");
    assert!(v.events[4].starts_with("record("));
}

// ── Error paths ─────────────────────────────────────────────────────────────

#[test]
fn test_visitor_invalid_magic() {
    let mut v = EventLog::default();
    let err = run_visitor(b"\x00\x61\x01", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::InvalidMagicNumber));
}

#[test]
fn test_visitor_truncated() {
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\x61", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_visitor_unknown_tag() {
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\xff", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnsupportedTag(0xff)));
}

#[test]
fn test_visitor_depth_limit() {
    // 129 nested tuples — exceeds the default max_depth of 128.
    let mut buf = vec![131u8];
    for _ in 0..129 {
        buf.push(104);
        buf.push(1);
    }
    buf.push(97);
    buf.push(0);
    let mut v = EventLog::default();
    let err = run_visitor(&buf, &mut v).unwrap_err();
    assert!(matches!(err, EtfError::RecursionLimitExceeded));
}

#[test]
fn test_visitor_atom_too_large() {
    // Use a tight `max_atom_len` so we can construct a valid small buffer
    // whose length exceeds it.  SMALL_ATOM_UTF8_EXT (119) is enough.
    let buf = vec![131, 119, 3, b'a', b'b', b'c']; // 3-byte atom
    let tight = Limits {
        max_atom_len: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::AtomTooLarge));
}

#[test]
fn test_visitor_string_too_large() {
    // STRING_EXT (107) with a length > max_string_len.  Use a tight limit so
    // we can construct a small buffer that trips the check.
    let buf = vec![131, 107, 0, 4, b'a', b'b', b'c', b'd']; // 4-byte string
    let tight = Limits {
        max_string_len: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_binary_too_large() {
    // BINARY_EXT (109) with a length > max_binary_size.  Use a tight limit
    // so we can trip the check with a small buffer.
    let buf = vec![131, 109, 0, 0, 0, 4, 1, 2, 3, 4]; // 4-byte binary
    let tight = Limits {
        max_binary_size: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_list_too_large() {
    // LIST_EXT (108) with len > max_list_len.  Use a tight limit.
    let buf = vec![131, 108, 0, 0, 0, 3, 97, 1, 97, 2, 97, 3, 106]; // 3-elem list
    let tight = Limits {
        max_list_len: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_map_too_large() {
    // MAP_EXT (116) with arity > max_map_len.  Use a tight limit.
    let buf = vec![131, 116, 0, 0, 0, 2, 97, 1, 97, 2, 97, 3, 97, 4]; // 2 pairs
    let tight = Limits {
        max_map_len: 1,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::MapTooLarge));
}

#[test]
fn test_visitor_tuple_too_large() {
    // LARGE_TUPLE_EXT (105) with arity > max_tuple_arity.  Use a tight limit.
    let buf = vec![131, 105, 0, 0, 0, 3, 97, 1, 97, 2, 97, 3]; // 3-tuple
    let tight = Limits {
        max_tuple_arity: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::MapTooLarge));
}

#[test]
fn test_visitor_small_big_too_large() {
    // LARGE_BIG_EXT (111) with len > max_binary_size.  Use a tight limit.
    let buf = vec![131, 111, 0, 0, 0, 4, 0, 1, 2, 3]; // 4-digit bignum
    let tight = Limits {
        max_binary_size: 3,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_ref_too_large() {
    // NEW_REFERENCE_EXT (114) with len > max_reference_words.  Use a tight
    // limit so the test runs in microseconds.
    let buf = vec![131, 114, 0, 3]; // 3 words
    let tight = Limits {
        max_reference_words: 2,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_visitor_new_fun_too_large() {
    // NEW_FUN_EXT: Size (after subtracting 4 for the Size field itself)
    // exceeds max_fun_size.  Use a tight limit.
    let buf = vec![131, 112, 0, 0, 0, 6]; // remaining = 2, limit 1
    let tight = Limits {
        max_fun_size: 1,
        ..Limits::default()
    };
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor(&buf, None, None, &mut v, &tight).unwrap_err();
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_visitor_invalid_fun_size() {
    // NEW_FUN_EXT: Size < 4 -> InvalidSize.
    let buf = vec![131, 112, 0, 0, 0, 3];
    let mut v = EventLog::default();
    let err = run_visitor(&buf, &mut v).unwrap_err();
    assert!(matches!(err, EtfError::InvalidSize));
}

#[test]
fn test_visitor_invalid_legacy_float() {
    // FLOAT_EXT (99): 31 bytes that don't form a parseable float.
    let mut buf = vec![131, 99];
    buf.extend(std::iter::repeat_n(b'x', 31));
    let mut v = EventLog::default();
    let err = run_visitor(&buf, &mut v).unwrap_err();
    assert!(matches!(err, EtfError::InvalidFloat));
}

#[test]
fn test_visitor_local_ext_unsupported() {
    // LOCAL_EXT (121) is reported as UnsupportedTag.
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\x79", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnsupportedTag(121)));
}

#[test]
fn test_visitor_atom_cache_ref_unsupported() {
    // ATOM_CACHE_REF (82) is reported as UnsupportedTag.
    let mut v = EventLog::default();
    let err = run_visitor(b"\x83\x52\x00", &mut v).unwrap_err();
    assert!(matches!(err, EtfError::UnsupportedTag(82)));
}

// ── Streaming visitor ───────────────────────────────────────────────────────

#[test]
fn test_visitor_streaming_complete() {
    let mut v = EventLog::default();
    parse_etf_with_visitor_streaming(b"\x83\x61\x2a", None, None, &mut v, &Limits::default())
        .unwrap();
    assert_eq!(v.events, vec!["int(42)"]);
}

#[test]
fn test_visitor_streaming_incomplete() {
    let mut v = EventLog::default();
    let err = parse_etf_with_visitor_streaming(b"\x83", None, None, &mut v, &Limits::default())
        .unwrap_err();
    assert!(matches!(err, EtfError::Incomplete(_)));
}

#[test]
fn test_visitor_streaming_invalid_magic() {
    let mut v = EventLog::default();
    let err =
        parse_etf_with_visitor_streaming(b"\x00\x61\x01", None, None, &mut v, &Limits::default())
            .unwrap_err();
    assert!(matches!(err, EtfError::InvalidMagicNumber));
}

// ── Default trait methods: a visitor that overrides nothing ─────────────────

/// A visitor that overrides no methods.  Every call falls through to the
/// default no-op implementations, exercising the default method bodies.
struct DefaultVisitor;

impl Visitor for DefaultVisitor {
    type Error = EtfError;
}

#[test]
fn test_visitor_default_methods_all_terms() {
    // A complex term that exercises every Visitor method (scalars, compound,
    // opaque wrappers) using only the default no-op implementations.
    //
    // {1, 2.5, "hello", <<1,2,3>>, 1024, "string", [4, 5], #{a => 1},
    //  <<1:3>>, PID, NEW_PID, PORT, NEW_PORT, V4_PORT, REF, NEWER_REF,
    //  EXPORT_FUN, NEW_FUN, RECORD, [1 | 2]}
    let mut buf = vec![131, 104, 19];
    // 1
    buf.extend_from_slice(b"\x61\x01");
    // 2.5 = NEW_FLOAT_EXT
    buf.extend_from_slice(b"\x46\x40\x04\x00\x00\x00\x00\x00\x00");
    // "hello" = SMALL_ATOM_UTF8_EXT
    buf.extend_from_slice(b"\x77\x05hello");
    // <<1,2,3>> = BINARY_EXT
    buf.extend_from_slice(b"\x6d\x00\x00\x00\x03\x01\x02\x03");
    // 256 = INTEGER_EXT
    buf.extend_from_slice(b"\x62\x00\x00\x01\x00");
    // "abc" = STRING_EXT
    buf.extend_from_slice(b"\x6b\x00\x03abc");
    // [4, 5] = LIST_EXT len=2 + tail nil
    buf.extend_from_slice(b"\x6c\x00\x00\x00\x02\x61\x04\x61\x05\x6a");
    // #{a => 1} = MAP_EXT
    buf.extend_from_slice(b"\x74\x00\x00\x00\x01\x77\x01a\x61\x01");
    // <<1:3>> = BIT_BINARY_EXT
    buf.extend_from_slice(b"\x4d\x00\x00\x00\x01\x03\x80");
    // PID_EXT
    buf.extend_from_slice(b"\x67\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x01");
    // NEW_PID_EXT
    buf.extend_from_slice(b"\x58\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01");
    // PORT_EXT
    buf.extend_from_slice(b"\x66\x77\x04node\x00\x00\x00\x01\x01");
    // NEW_PORT_EXT
    buf.extend_from_slice(b"\x59\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01");
    // V4_PORT_EXT
    buf.extend_from_slice(b"\x78\x77\x04node\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x01");
    // NEW_REFERENCE_EXT
    buf.extend_from_slice(b"\x72\x00\x01\x77\x04node\x01\x00\x00\x00\x01");
    // NEWER_REFERENCE_EXT
    buf.extend_from_slice(b"\x5a\x00\x01\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01");
    // EXPORT_EXT
    buf.extend_from_slice(b"\x71\x77\x05lists\x77\x03map\x61\x02");
    // NEW_FUN_EXT
    let fun_payload = [
        1, // arity
        // 16-byte uniq
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 4-byte index
        0, 0, 0, 0, // 4-byte num_free
        0, 0, 0, 0, // module atom "mod"
        0x77, 3, b'm', b'o', b'd', // old_index = int(0)
        0x61, 0, // old_uniq = int(0)
        0x61, 0, // pid
        0x67, 0x77, 4, b'n', b'o', b'd', b'e', 0, 0, 0, 1, 0, 0, 0, 1, 1,
    ];
    buf.push(0x70);
    let size = (fun_payload.len() + 4) as u32;
    buf.extend_from_slice(&size.to_be_bytes());
    buf.extend_from_slice(&fun_payload);
    // RECORD_EXT: 1 field
    buf.extend_from_slice(b"\x43\x00\x00\x00\x01\x01\x77\x03foo\x77\x03bar\x77\x01x\x61\x2a");
    // improper list [1 | 2]
    buf.extend_from_slice(b"\x6c\x00\x00\x00\x01\x61\x01\x61\x02");
    // Bigint = SMALL_BIG_EXT 1 byte 0
    buf.extend_from_slice(b"\x6e\x01\x00\x00");
    // LARGE_BIG_EXT
    buf.extend_from_slice(b"\x6f\x00\x00\x00\x01\x00\x00");
    let mut v = DefaultVisitor;
    parse_etf_with_visitor(&buf, None, None, &mut v, &Limits::default()).unwrap();
}

#[test]
fn test_visitor_default_improper_list_and_big_int() {
    // Improper list [1 | 2] and SMALL_BIG_EXT/LARGE_BIG_EXT in isolation.
    let mut buf = vec![131, 104, 3];
    // [1 | 2]
    buf.extend_from_slice(b"\x6c\x00\x00\x00\x01\x61\x01\x61\x02");
    // SMALL_BIG_EXT
    buf.extend_from_slice(b"\x6e\x01\x00\xab");
    // LARGE_BIG_EXT
    buf.extend_from_slice(b"\x6f\x00\x00\x00\x01\x00\xab");
    let mut v = DefaultVisitor;
    parse_etf_with_visitor(&buf, None, None, &mut v, &Limits::default()).unwrap();
}
