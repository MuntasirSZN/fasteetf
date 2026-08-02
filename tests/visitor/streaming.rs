use super::*;

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
