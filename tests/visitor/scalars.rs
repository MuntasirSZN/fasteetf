use super::*;

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
