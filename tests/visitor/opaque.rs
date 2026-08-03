use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

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
