use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

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
