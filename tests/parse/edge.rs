use super::*;

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn test_zero_length_binary() {
    with_parse(b"\x83\x6d\x00\x00\x00\x00", |term| {
        assert!(matches!(term, Term::Binary(b) if b.is_empty()));
    });
}

#[test]
fn test_atom_max_length() {
    let mut buf = vec![131, 119, 255];
    buf.extend(std::iter::repeat_n(b'a', 255));
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Atom(_)));
    });
}

#[test]
fn test_max_depth_ok() {
    let mut buf = vec![131u8];
    for _ in 0..128 {
        buf.push(104);
        buf.push(1);
    }
    buf.push(97);
    buf.push(0);
    with_parse(&buf, |term| match term {
        Term::Tuple(_) => {}
        _ => panic!("expected Tuple at depth 128"),
    });
}
