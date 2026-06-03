// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for streaming / incremental parsing and the Needed API.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

use core::mem::MaybeUninit;
use fasteetf::*;

#[test]
fn test_streaming_incomplete() {
    // Feed the magic byte only — the parser should demand at least 1 more.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83",
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: Limits::default(),
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_incomplete_tag() {
    // 131 + nothing — need the tag byte.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83",
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),
    })
    .unwrap_err();
    match err {
        fasteetf::EtfError::Incomplete(needed) => {
            assert!(needed.size().unwrap_or(0) >= 1);
        }
        _ => panic!("expected Incomplete"),
    }
}

#[test]
fn test_streaming_full_parse() {
    // Complete input should succeed identically to parse_etf.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let term = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x61\x2a",
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),
    })
    .unwrap();
    assert!(matches!(term, fasteetf::Term::Int(42)));
}

#[test]
fn test_streaming_accumulate() {
    // Simulate receiving data in chunks.
    let full = b"\x83\x68\x03\x61\x01\x61\x02\x61\x03"; // tuple(1, 2, 3)
    for split in 1..full.len() {
        let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
        let chunk = &full[..split];
        // First attempt should fail with Incomplete (unless split == full.len).
        if split < full.len() {
            let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
                input: chunk,
                decompressed_buffer: None,
                ast_arena: &mut arena,
                limits: fasteetf::Limits::default(),
            })
            .unwrap_err();
            assert!(
                matches!(err, fasteetf::EtfError::Incomplete(_)),
                "split={split}: expected Incomplete, got {err:?}"
            );
        }
        // With the full input it must succeed.
        let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
        let term = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
            input: full,
            decompressed_buffer: None,
            limits: fasteetf::Limits::default(),
            ast_arena: &mut arena,
        })
        .unwrap();
        match term {
            fasteetf::Term::Tuple(elems) => assert_eq!(elems.len(), 3),
            _ => panic!("expected Tuple"),
        }
    }
}

#[test]
fn test_streaming_needed_size() {
    // SMALL_INTEGER_EXT needs 2 bytes after magic: tag + value.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83",
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),
    })
    .unwrap_err();
    match err {
        fasteetf::EtfError::Incomplete(n) => {
            // After magic we need at least 1 byte (the tag).
            assert!(n.size().unwrap_or(0) >= 1);
        }
        _ => panic!("expected Incomplete"),
    }
}

#[test]
fn test_streaming_visitor() {
    use fasteetf::Visitor;
    struct IntCatcher(Option<i32>);
    impl Visitor for IntCatcher {
        type Error = fasteetf::EtfError;
        fn visit_int(&mut self, value: i32) -> Result<(), Self::Error> {
            self.0 = Some(value);
            Ok(())
        }
    }

    let mut v = IntCatcher(None);
    let err = fasteetf::parse_etf_with_visitor_streaming(b"\x83", None, &mut v, &Limits::default())
        .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));

    let mut v = IntCatcher(None);
    fasteetf::parse_etf_with_visitor_streaming(b"\x83\x61\x2a", None, &mut v, &Limits::default())
        .unwrap();
    assert_eq!(v.0, Some(42));
}

#[test]
fn test_needed_api() {
    let n = fasteetf::Needed::Size(42);
    assert_eq!(n.size(), Some(42));
    assert!(n.is_exact());

    let u = fasteetf::Needed::Unknown;
    assert_eq!(u.size(), None);
    assert!(!u.is_exact());
}
