// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for resource-limit enforcement via [`Limits`] and for
// parser/encoder error paths that are unreachable with the default limits.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

#[path = "../common/mod.rs"]
mod common;
use common::*;
use core::mem::MaybeUninit;
use fasteetf::*;

/// Parse `input` with a custom `Limits`, returning the error.
fn parse_err_with_limits(input: &[u8], limits: Limits) -> EtfError {
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 65536];
    #[cfg(feature = "compression")]
    let options = ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits,
        zlib_backend: None,
    };
    #[cfg(not(feature = "compression"))]
    let options = ParseOptions {
        input,
        ast_arena: &mut arena,
        limits,
    };
    parse_etf(options).unwrap_err()
}

// ── Limit presets ───────────────────────────────────────────────────────────

#[test]
fn test_limits_embedded() {
    let l = Limits::embedded();
    assert_eq!(l.max_binary_size, 64 * 1024);
    assert_eq!(l.max_bit_binary_size, 64 * 1024);
    assert_eq!(l.max_list_len, 1024);
    assert_eq!(l.max_map_len, 1024);
    assert_eq!(l.max_atom_len, 255);
    assert_eq!(l.max_tuple_arity, 256);
    assert_eq!(l.max_string_len, 1024);
    assert_eq!(l.max_reference_words, 5);
    assert_eq!(l.max_depth, 32);
    assert_eq!(l.max_fun_size, 64 * 1024);
    assert_eq!(l.max_bignum_size, 1024);
    assert!(l.expand_string_ext_to_list);
}

#[test]
fn test_limits_relaxed() {
    let l = Limits::relaxed();
    assert_eq!(l.max_binary_size, 256 * 1024 * 1024);
    assert_eq!(l.max_bit_binary_size, 256 * 1024 * 1024);
    assert_eq!(l.max_list_len, 10_000_000);
    assert_eq!(l.max_map_len, 10_000_000);
    assert_eq!(l.max_atom_len, 65_535);
    assert_eq!(l.max_tuple_arity, 10_000_000);
    assert_eq!(l.max_string_len, 65_535);
    assert_eq!(l.max_reference_words, 5);
    assert_eq!(l.max_depth, 256);
    assert_eq!(l.max_fun_size, 256 * 1024 * 1024);
    assert_eq!(l.max_bignum_size, 256 * 1024 * 1024);
    assert!(l.expand_string_ext_to_list);
}

// ── Atom limits ─────────────────────────────────────────────────────────────

#[test]
fn test_atom_utf8_ext_too_large() {
    // ATOM_UTF8_EXT (118) with a 3-byte name vs max_atom_len = 2.
    let limits = Limits {
        max_atom_len: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x76\x00\x03abc", limits);
    assert!(matches!(err, EtfError::AtomTooLarge));
}

#[test]
fn test_small_atom_utf8_too_large() {
    // SMALL_ATOM_UTF8_EXT (119) with a 3-byte name vs max_atom_len = 2.
    let limits = Limits {
        max_atom_len: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x77\x03abc", limits);
    assert!(matches!(err, EtfError::AtomTooLarge));
}

// ── parse_atom_only (atom fields inside opaque terms) ───────────────────────

#[test]
fn test_ref_node_atom_utf8_ext() {
    // NEW_REFERENCE_EXT whose node field is an ATOM_UTF8_EXT (118) atom —
    // exercises parse_atom_only's ATOM_UTF8_EXT branch.
    let buf = [
        131, 114, 0, 1, 118, 0, 4, b'n', b'o', b'd', b'e', 1, 0, 0, 0, 1,
    ];
    with_parse(&buf, |term| assert!(matches!(term, Term::Ref(_))));
}

#[test]
fn test_ref_node_small_atom_too_large() {
    // parse_atom_only: node encoded as SMALL_ATOM_UTF8_EXT exceeding the limit.
    let buf = [131, 114, 0, 1, 119, 3, b'a', b'b', b'c', 1, 0, 0, 0, 1];
    let limits = Limits {
        max_atom_len: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(&buf, limits);
    assert!(matches!(err, EtfError::AtomTooLarge));
}

#[test]
fn test_ref_node_invalid_atom_field() {
    // parse_atom_only: the node field is an integer, not an atom.
    let buf = [131, 114, 0, 1, 97, 1, 1, 0, 0, 0, 1];
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::InvalidAtomField));
}

// ── Bignum limits ───────────────────────────────────────────────────────────

#[test]
fn test_small_big_too_large() {
    // SMALL_BIG_EXT (110): 3 digits vs max_bignum_size = 2.
    let limits = Limits {
        max_bignum_size: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x6e\x03\x00\x01\x02\x03", limits);
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_large_big_too_large() {
    // LARGE_BIG_EXT (111): 3 digits vs max_bignum_size = 2.
    let limits = Limits {
        max_bignum_size: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x6f\x00\x00\x00\x03\x00\x01\x02\x03", limits);
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

// ── Compound-term limits ────────────────────────────────────────────────────

#[test]
fn test_large_tuple_too_large() {
    // LARGE_TUPLE_EXT (105): arity 3 vs max_tuple_arity = 2.
    let limits = Limits {
        max_tuple_arity: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x69\x00\x00\x00\x03\x61\x01\x61\x02\x61\x03", limits);
    assert!(matches!(err, EtfError::TupleTooLarge));
}

#[test]
fn test_string_ext_too_large() {
    // STRING_EXT (107): 3 chars vs max_string_len = 2.
    let limits = Limits {
        max_string_len: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x6b\x00\x03abc", limits);
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_list_too_large() {
    // LIST_EXT (108): 3 elements vs max_list_len = 2.
    let limits = Limits {
        max_list_len: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(
        b"\x83\x6c\x00\x00\x00\x03\x61\x01\x61\x02\x61\x03\x6a",
        limits,
    );
    assert!(matches!(err, EtfError::ListTooLarge));
}

#[test]
fn test_map_too_large() {
    // MAP_EXT (116): 2 pairs vs max_map_len = 1.
    let limits = Limits {
        max_map_len: 1,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x74\x00\x00\x00\x02", limits);
    assert!(matches!(err, EtfError::MapTooLarge));
}

#[test]
fn test_binary_too_large() {
    // BINARY_EXT (109): 3 bytes vs max_binary_size = 2.
    let limits = Limits {
        max_binary_size: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x6d\x00\x00\x00\x03", limits);
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

// ── Opaque-term limits ──────────────────────────────────────────────────────

#[test]
fn test_ref_legacy_too_large() {
    // NEW_REFERENCE_EXT (114): 6 ID words vs max_reference_words = 5.
    let limits = Limits {
        max_reference_words: 5,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x72\x00\x06", limits);
    assert!(matches!(err, EtfError::ReferenceTooLarge));
}

#[test]
fn test_ref_newer_too_large() {
    // NEWER_REFERENCE_EXT (90): 6 ID words vs max_reference_words = 5.
    let limits = Limits {
        max_reference_words: 5,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x5a\x00\x06", limits);
    assert!(matches!(err, EtfError::ReferenceTooLarge));
}

#[test]
fn test_new_fun_too_large() {
    // NEW_FUN_EXT (112): Size 6 → 2 payload bytes vs max_fun_size = 1.
    let limits = Limits {
        max_fun_size: 1,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x70\x00\x00\x00\x06", limits);
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_record_too_large() {
    // RECORD_EXT (67): 3 fields vs max_map_len = 2.
    let limits = Limits {
        max_map_len: 2,
        ..Limits::default()
    };
    let err = parse_err_with_limits(b"\x83\x43\x00\x00\x00\x03", limits);
    assert!(matches!(err, EtfError::MapTooLarge));
}

// ── Unsupported tags in the parser dispatch ─────────────────────────────────

#[test]
fn test_local_ext_unsupported() {
    // LOCAL_EXT (121) is a distribution-protocol tag, never a standalone term.
    let err = parse_err(b"\x83\x79\x00");
    assert!(matches!(err, EtfError::UnsupportedTag(121)));
}

#[test]
fn test_atom_cache_ref_unsupported() {
    // ATOM_CACHE_REF (82) is distribution-only.
    let err = parse_err(b"\x83\x52\x01");
    assert!(matches!(err, EtfError::UnsupportedTag(82)));
}

#[test]
fn test_nested_compressed_unsupported() {
    // COMPRESSED (80) is only valid as the outer wrapper; nested it is an
    // unsupported tag inside the term grammar.
    let err = parse_err(b"\x83\x68\x01\x50");
    assert!(matches!(err, EtfError::UnsupportedTag(80)));
}

// ── Truncated primitive reads ───────────────────────────────────────────────

#[test]
fn test_truncated_new_float() {
    // NEW_FLOAT_EXT (70) with only 7 of the 8 payload bytes present.
    let err = parse_err(b"\x83\x46\x01\x02\x03\x04\x05\x06\x07");
    assert!(matches!(err, EtfError::UnexpectedEof));
}

// ── Arena alignment ─────────────────────────────────────────────────────────

#[test]
fn test_parse_with_misaligned_arena() {
    // The bump allocator must align its start pointer up to max_align_t even
    // when handed a misaligned buffer (offset by one byte here).
    let mut buf = vec![MaybeUninit::<u8>::uninit(); 256];
    let mis = &mut buf[1..];
    let input = b"\x83\x68\x02\x61\x01\x61\x02";
    #[cfg(feature = "compression")]
    let options = ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: mis,
        limits: Limits::default(),
        zlib_backend: None,
    };
    #[cfg(not(feature = "compression"))]
    let options = ParseOptions {
        input,
        ast_arena: mis,
        limits: Limits::default(),
    };
    let term = parse_etf(options).unwrap();
    assert!(matches!(term, Term::Tuple(_)));
}
