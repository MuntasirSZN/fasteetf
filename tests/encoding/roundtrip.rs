use super::*;

// ── Comprehensive roundtrip: encode → parse ───────────────────────────────

#[test]
fn test_roundtrip_int() {
    for &v in &[0i32, 1, 127, 128, 255, 256, -1, i32::MIN, i32::MAX] {
        let encoded = encode_ok(&Term::Int(v));
        with_parse(&encoded, |parsed| {
            assert!(
                matches!(parsed, Term::Int(x) if x == v),
                "mismatch for {}",
                v
            );
        });
    }
}

#[test]
fn test_roundtrip_float() {
    for &v in &[0.0f64, 1.0, -1.0, std::f64::consts::PI, 1.0e200, -2.5e-100] {
        let encoded = encode_ok(&Term::Float(v));
        with_parse(&encoded, |parsed| {
            assert!(
                matches!(parsed, Term::Float(x) if x == v),
                "mismatch for {}",
                v
            );
        });
    }
}

#[test]
fn test_roundtrip_list() {
    // Empty
    let encoded = encode_ok(&Term::List(&[]));
    with_parse(&encoded, |parsed| {
        assert!(matches!(parsed, Term::List(&[])));
    });

    // Non-empty
    let terms = [Term::Int(1), Term::Int(2), Term::Int(3)];
    let encoded = encode_ok(&Term::List(&terms));
    with_parse(&encoded, |parsed| match parsed {
        Term::List(l) => assert_eq!(l.len(), 3),
        _ => panic!("expected List"),
    });
}

#[test]
fn test_roundtrip_nested() {
    // Build a nested term via parse-then-encode.
    let input = b"\x83\x68\x02\x61\x01\x68\x02\x61\x02\x61\x03"; // {1, {2, 3}}
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_complex() {
    // Parse a complex term, encode it, and verify the encoded bytes match.
    let input = b"\x83\x74\x00\x00\x00\x02\x61\x01\x61\x02\x61\x03\x61\x04";
    // MAP #{1=>2, 3=>4}
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_pid() {
    // Build a PID term via parse (PID_EXT).
    // Wire: 103 Node=atom("node") ID=1 Serial=1 Creation=1
    let input = b"\x83\x67\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_new_pid() {
    // NEW_PID_EXT with 4-byte Creation.
    let input = b"\x83\x58\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_port_v4() {
    // V4_PORT_EXT.
    // Wire: 120 Node="node" ID=1 Creation=1
    let input = b"\x83\x78\x77\x04node\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_export() {
    // EXPORT_EXT: fun lists:map/2
    // Wire: 113 Module=atom("lists") Function=atom("map") Arity=int(2)
    let input = b"\x83\x71\x77\x05lists\x77\x03map\x61\x02";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_new_fun() {
    // NEW_FUN_EXT: fun with no free variables.
    // Wire: 112 Size(4) Arity(1) Uniq(16) Index(4) NumFree(4) Module OldIndex OldUniq Pid
    //
    // Breakdown of the payload bytes after Size:
    //   1  (Arity)
    // + 16 (Uniq)
    // + 4  (Index)
    // + 4  (NumFree)
    // + 3  (Module atom: tag 77 + len 1 + 'm')
    // + 2  (OldIndex: SMALL_INTEGER_EXT + 0)
    // + 2  (OldUniq: SMALL_INTEGER_EXT + 0)
    // + 16 (NEW_PID_EXT: tag 88 + atom "n" (3) + ID(4) + Serial(4) + Creation(4))
    //   = 48 bytes after Size
    // Size (includes the 4-byte Size field) = 4 + 48 = 52 = 0x34
    //
    // Full input (54 bytes):
    //   magic(1) + tag(1) + Size(4) + payload(48)
    let input = b"\x83\x70\x00\x00\x00\x34\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x77\x01\x6d\x61\x00\x61\x00\x58\x77\x01\x6e\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_record() {
    // RECORD_EXT: a native record.
    // Wire: 67 #Fields=1 Flags=1 Module=foo Name=bar FieldNames=[x] Values=[42]
    // We need to construct a valid record. Parse one.
    let input = b"\x83\x43\x00\x00\x00\x01\x01\x77\x03foo\x77\x03bar\x77\x01x\x61\x2a";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_bit_binary() {
    let input = b"\x83\x4d\x00\x00\x00\x02\x07\xab\xcd";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_reference() {
    // NEW_REFERENCE_EXT.
    // Wire: 114 Len=1 Node="node" Creation=1 ID=[1]
    let input = b"\x83\x72\x00\x01\x77\x04node\x01\x00\x00\x00\x01";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}

#[test]
fn test_roundtrip_newer_reference() {
    // NEWER_REFERENCE_EXT.
    let input = b"\x83\x5a\x00\x01\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x02";
    let encoded = with_parse(input, |term| encode_ok(&term));
    assert_eq!(encoded, input);
}
