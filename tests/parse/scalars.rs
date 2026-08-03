use super::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

// ── Scalars ─────────────────────────────────────────────────────────────────

#[test]
fn test_small_integer() {
    with_parse(b"\x83\x61\x2a", |term| {
        assert!(matches!(term, Term::Int(42)));
    });
}

#[test]
fn test_integer_ext() {
    with_parse(b"\x83\x62\x00\x00\x00\xff", |term| {
        assert!(matches!(term, Term::Int(255)));
    });
}

#[test]
fn test_negative_integer() {
    with_parse(b"\x83\x62\xff\xff\xff\xff", |term| {
        assert!(matches!(term, Term::Int(-1)));
    });
}

#[test]
fn test_float() {
    with_parse(b"\x83\x46\x40\x09\x21\xfb\x54\x44\x2d\x18", |term| {
        assert!(matches!(term, Term::Float(v) if (v - core::f64::consts::PI).abs() < 1e-12));
    });
}

#[test]
fn test_legacy_float() {
    let content = format!("{:<30.20e}", 42.0f64);
    assert_eq!(content.len(), 30);
    let mut buf = Vec::with_capacity(33);
    buf.push(131);
    buf.push(99);
    buf.extend_from_slice(content.as_bytes());
    buf.push(0);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Float(v) if (v - 42.0).abs() < 1e-12));
    });
}

#[test]
fn test_small_big() {
    with_parse(b"\x83\x6e\x02\x00\x00\x01", |term| {
        assert!(matches!(term, Term::BigInt { sign: 0, digits } if digits == [0, 1]));
    });
}

#[test]
fn test_large_big() {
    with_parse(&[131, 111, 0, 0, 0, 1, 0, 42], |term| {
        assert!(matches!(term, Term::BigInt { sign: 0, digits } if digits == [42]));
    });
}
