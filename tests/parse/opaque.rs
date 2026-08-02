use super::*;

// ── PIDs ────────────────────────────────────────────────────────────────────

#[test]
fn test_pid_ext() {
    let mut buf = vec![131, 103];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 2]);
    buf.push(0);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Pid(_)));
    });
}

#[test]
fn test_new_pid_ext() {
    let mut buf = vec![131, 88];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 2]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Pid(_)));
    });
}

// ── Ports ───────────────────────────────────────────────────────────────────

#[test]
fn test_port_ext() {
    let mut buf = vec![131, 102];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.push(0);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Port(_)));
    });
}

#[test]
fn test_v4_port_ext() {
    let mut buf = vec![131, 120];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Port(_)));
    });
}

// ── References ──────────────────────────────────────────────────────────────

#[test]
fn test_new_reference_ext() {
    let mut buf = vec![131, 114, 0, 1];
    buf.extend_from_slice(b"\x77\x04node");
    buf.push(0);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Ref(_)));
    });
}

#[test]
fn test_newer_reference_ext() {
    let mut buf = vec![131, 90, 0, 1];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Ref(_)));
    });
}

// ── Functions ───────────────────────────────────────────────────────────────

#[test]
fn test_export_ext() {
    let mut buf = vec![131, 113];
    buf.extend_from_slice(b"\x77\x03mod");
    buf.extend_from_slice(b"\x77\x04func");
    buf.extend_from_slice(b"\x61\x02");
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Function(_)));
    });
}
