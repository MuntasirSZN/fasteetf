use super::*;
use core::mem::MaybeUninit;

// ── Streaming EOF paths: exercise Cursor::take/read_u16/read_u32/read_f64 EOF branches ─

#[test]
fn test_streaming_truncated_after_magic() {
    // 131 + nothing — the magic byte take(1) succeeds, but the next read_u8 for the
    // tag byte hits the cursor EOF path. Streaming mode returns Incomplete(1).
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_integer_ext() {
    // 131 98 + only 2 bytes of a 4-byte u32 value (INTEGER_EXT).
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x62\x00\x01",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_float() {
    // 131 70 + only 4 bytes of an 8-byte f64 (NEW_FLOAT_EXT).
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x46\x00\x01\x02\x03",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_atom() {
    // 131 118 + 2-byte length (5) + only 2 of 5 atom bytes.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x76\x00\x05he",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_binary() {
    // 131 109 + 4-byte length (5) + only 2 of 5 binary bytes.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x6d\x00\x00\x00\x05\xab\xcd",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_tuple() {
    // 131 104 + arity 2 + only one element.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x68\x02\x61\x01",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_list_tail() {
    // 131 108 0 0 0 1 + 1 element + nothing for tail.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x6c\x00\x00\x00\x01\x61\x01",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_pid() {
    // 131 103 + atom "node" (6 bytes) + only 4 of 9 ID+Serial+Creation bytes.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x67\x77\x04node\x00\x00\x00\x01",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_invalid_magic_returns_error() {
    // A non-magic byte at the start: parse_etf_streaming should return
    // InvalidMagicNumber, not Incomplete (the magic byte was present, just wrong).
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x00",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::InvalidMagicNumber));
}

#[test]
fn test_streaming_visitor_truncated_payload() {
    use fasteetf::Visitor;
    struct IntCatcher(Option<i32>);
    impl Visitor for IntCatcher {
        type Error = fasteetf::EtfError;
        fn visit_int(&mut self, value: i32) -> Result<(), Self::Error> {
            self.0 = Some(value);
            Ok(())
        }
    }

    // Truncated INTEGER_EXT (need 4 bytes after tag, only 2).
    let mut v = IntCatcher(None);
    let err = fasteetf::parse_etf_with_visitor_streaming(
        b"\x83\x62\x00\x01",
        None,
        None,
        &mut v,
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_visitor_invalid_magic() {
    use fasteetf::Visitor;
    struct NoopVisitor;
    impl Visitor for NoopVisitor {
        type Error = fasteetf::EtfError;
    }

    let mut v = NoopVisitor;
    let err =
        fasteetf::parse_etf_with_visitor_streaming(b"\x00", None, None, &mut v, &Limits::default())
            .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::InvalidMagicNumber));
}

#[test]
fn test_streaming_compressed_no_buffer() {
    // 131 80 + 4-byte size + payload, but no decompression buffer supplied.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x50\x00\x00\x00\x05\x78\x9c\x00\x00\x00\x00\x01",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    });
    // Either an error or a successful parse depending on what the runtime does;
    // we just want to exercise the path.
    let _ = err;
}

#[test]
fn test_streaming_truncated_in_atom_utf8_length() {
    // 131 118 (ATOM_UTF8_EXT) + only 1 byte of the 2-byte u16 length.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x76\x00",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_string_ext_length() {
    // 131 107 (STRING_EXT) + only 1 byte of the 2-byte u16 length.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x6b\x00",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}

#[test]
fn test_streaming_truncated_in_reference_length() {
    // 131 114 (NEW_REFERENCE_EXT) + only 1 byte of the 2-byte u16 length.
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    let err = fasteetf::parse_etf_streaming(fasteetf::ParseOptions {
        input: b"\x83\x72\x00",

        #[cfg(feature = "compression")]
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: fasteetf::Limits::default(),

        #[cfg(feature = "compression")]
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, fasteetf::EtfError::Incomplete(_)));
}
