// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for ETF parsing — every tag variant with valid input,
// error cases, OwnedTerm conversion, atom ergonomics, and edge cases.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

mod common;
use common::*;
use fasteetf::*;

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
        assert!(matches!(term, Term::SmallBigInt { sign: 0, digits } if digits == [0, 1]));
    });
}

#[test]
fn test_large_big() {
    with_parse(&[131, 111, 0, 0, 0, 1, 0, 42], |term| {
        assert!(matches!(term, Term::LargeBigInt { sign: 0, digits } if digits == [42]));
    });
}

// ── Atoms (lazy UTF-8) ──────────────────────────────────────────────────────

#[test]
fn test_atom_utf8() {
    with_parse(b"\x83\x76\x00\x05hello", |term| match term {
        Term::Atom(a) => {
            assert_eq!(a.as_str(), Ok("hello"));
            assert_eq!(a.as_bytes(), b"hello");
        }
        _ => panic!("expected Atom"),
    });
}

#[test]
fn test_small_atom_utf8() {
    with_parse(b"\x83\x77\x03hi!", |term| match term {
        Term::Atom(a) => {
            assert_eq!(a.as_str(), Ok("hi!"));
        }
        _ => panic!("expected Atom"),
    });
}

#[test]
fn test_empty_atom() {
    with_parse(b"\x83\x77\x00", |term| match term {
        Term::Atom(a) => {
            assert!(a.is_empty());
            assert_eq!(a.len(), 0);
        }
        _ => panic!("expected Atom"),
    });
}

#[test]
fn test_atom_lazy_utf8() {
    with_parse(b"\x83\x77\x02\xff\xfe", |term| match term {
        Term::Atom(a) => {
            assert!(a.as_str().is_err());
            assert_eq!(a.as_bytes(), &[0xff, 0xfe]);
        }
        _ => panic!("expected Atom"),
    });
}

// ── Tuples ──────────────────────────────────────────────────────────────────

#[test]
fn test_empty_tuple() {
    with_parse(b"\x83\x68\x00", |term| {
        assert!(matches!(term, Term::Tuple(&[])));
    });
}

#[test]
fn test_small_tuple() {
    with_parse(b"\x83\x68\x02\x61\x05\x61\x0a", |term| match term {
        Term::Tuple(elems) => {
            assert_eq!(elems.len(), 2);
            assert!(matches!(elems[0], Term::Int(5)));
            assert!(matches!(elems[1], Term::Int(10)));
        }
        _ => panic!("expected Tuple"),
    });
}

#[test]
fn test_large_tuple() {
    with_parse(&[131, 105, 0, 0, 0, 2, 97, 1, 97, 2], |term| match term {
        Term::Tuple(elems) => {
            assert_eq!(elems.len(), 2);
        }
        _ => panic!("expected Tuple"),
    });
}

// ── Lists ───────────────────────────────────────────────────────────────────

#[test]
fn test_nil() {
    with_parse(b"\x83\x6a", |term| match term {
        Term::List(elems) => assert!(elems.is_empty()),
        _ => panic!("expected List"),
    });
}

#[test]
fn test_string_ext() {
    with_parse(b"\x83\x6b\x00\x04\x41\x42\x43\x44", |term| match term {
        Term::List(elems) => {
            assert_eq!(elems.len(), 4);
            assert!(matches!(elems[0], Term::Int(65)));
        }
        _ => panic!("expected List"),
    });
}

#[test]
fn test_proper_list() {
    with_parse(
        &vec![131, 108, 0, 0, 0, 3, 97, 10, 97, 20, 97, 30, 106],
        |term| match term {
            Term::List(elems) => {
                assert_eq!(elems.len(), 3);
            }
            _ => panic!("expected List"),
        },
    );
}

#[test]
fn test_improper_list() {
    with_parse(&[131, 108, 0, 0, 0, 1, 97, 1, 97, 2], |term| match term {
        Term::ImproperList { elements, tail } => {
            assert_eq!(elements.len(), 1);
            assert!(matches!(elements[0], Term::Int(1)));
            assert!(matches!(tail, Term::Int(2)));
        }
        _ => panic!("expected ImproperList"),
    });
}

// ── Maps ────────────────────────────────────────────────────────────────────

#[test]
fn test_empty_map() {
    with_parse(&[131, 116, 0, 0, 0, 0], |term| match term {
        Term::Map(pairs) => assert!(pairs.is_empty()),
        _ => panic!("expected Map"),
    });
}

#[test]
fn test_map() {
    let mut buf = vec![131, 116, 0, 0, 0, 2];
    buf.extend_from_slice(b"\x61\x01");
    buf.extend_from_slice(b"\x77\x01\x61");
    buf.extend_from_slice(b"\x61\x02");
    buf.extend_from_slice(b"\x77\x01\x62");
    with_parse(&buf, |term| match term {
        Term::Map(pairs) => {
            assert_eq!(pairs.len(), 2);
        }
        _ => panic!("expected Map"),
    });
}

// ── Binaries ────────────────────────────────────────────────────────────────

#[test]
fn test_empty_binary() {
    with_parse(&[131, 109, 0, 0, 0, 0], |term| {
        assert!(matches!(term, Term::Binary(b) if b.is_empty()));
    });
}

#[test]
fn test_binary() {
    let mut buf = vec![131, 109, 0, 0, 0, 4];
    buf.extend_from_slice(b"data");
    with_parse(&buf, |term| {
        assert!(matches!(term, Term::Binary(b) if b == b"data"));
    });
}

#[test]
fn test_bit_binary() {
    with_parse(&[131, 77, 0, 0, 0, 1, 3, 0b1010_0000], |term| {
        assert!(matches!(term, Term::BitBinary { bits: 3, data } if data == [0b1010_0000]));
    });
}

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

// ── Error cases ─────────────────────────────────────────────────────────────

#[test]
fn test_invalid_magic() {
    let err = parse_err(b"\x00\x61\x01");
    assert!(matches!(err, EtfError::InvalidMagicNumber));
}

#[test]
fn test_truncated() {
    let err = parse_err(b"\x83\x61");
    assert!(matches!(err, EtfError::UnexpectedEof));
}

#[test]
fn test_unknown_tag() {
    let err = parse_err(b"\x83\xff");
    assert!(matches!(err, EtfError::UnsupportedTag(255)));
}

#[test]
fn test_depth_limit() {
    let mut buf = vec![131u8];
    for _ in 0..129 {
        buf.push(104);
        buf.push(1);
    }
    buf.push(97);
    buf.push(0);
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::RecursionLimitExceeded));
}

#[test]
fn test_binary_too_large() {
    let buf = vec![131, 109, 4, 16, 0, 0];
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::BinaryTooLarge));
}

#[test]
fn test_arena_exhaustion() {
    use core::mem::MaybeUninit;
    let mut arena = vec![MaybeUninit::<u8>::uninit(); 16];
    let input = b"\x83\x68\x0a\x61\x01\x61\x02\x61\x03\x61\x04\x61\x05\
                  \x61\x06\x61\x07\x61\x08\x61\x09\x61\x0a";
    let err = parse_etf(ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &mut arena,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap_err();
    assert!(matches!(err, EtfError::ArenaExhausted));
}

#[test]
fn test_invalid_fun_size() {
    let buf = vec![131, 112, 0, 0, 0, 3];
    let err = parse_err(&buf);
    assert!(matches!(err, EtfError::InvalidSize));
}

// ── OwnedTerm conversion ────────────────────────────────────────────────────

#[test]
fn test_owned_conversion() {
    use fasteetf::owned::OwnedTerm;
    with_parse(b"\x83\x61\x2a", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Int(42) => {}
            _ => panic!("expected Int(42)"),
        }
    });
}

#[test]
fn test_owned_list() {
    use fasteetf::owned::OwnedTerm;
    with_parse(b"\x83\x68\x02\x61\x01\x61\x02", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Tuple(ref elems) => {
                assert_eq!(elems.len(), 2);
            }
            _ => panic!("expected Tuple"),
        }
    });
}

// ── AtomUtf8 ergonomics ─────────────────────────────────────────────────────

#[test]
fn test_atom_utf8_comparison() {
    with_parse(b"\x83\x76\x00\x04true", |term| match term {
        Term::Atom(a) => {
            assert_eq!(a, "true");
            assert_eq!("true", a);
        }
        _ => panic!("expected Atom"),
    });
}

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

// ── New tags and conversions ────────────────────────────────────────────────

#[test]
fn test_new_port_ext() {
    // NEW_PORT_EXT (89): Node atom + 4-byte ID + 4-byte Creation.
    let mut buf = vec![131, 89];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    with_parse(&buf, |term| match term {
        Term::Port(Port(tag, data)) => {
            assert_eq!(tag, 89);
            assert_eq!(data.len(), 6 + 8); // 6 for the atom + 8 for ID+Creation
        }
        _ => panic!("expected Port"),
    });
}

#[test]
fn test_owned_atom_conversion() {
    use fasteetf::owned::OwnedTerm;
    with_parse(b"\x83\x77\x05hello", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Atom(s) => assert_eq!(s, "hello"),
            _ => panic!("expected Atom, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_atom_invalid_utf8_lossy() {
    // Invalid-UTF8 bytes should round-trip as lossy string.
    use fasteetf::owned::OwnedTerm;
    with_parse(b"\x83\x77\x02\xff\xfe", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Atom(s) => {
                // The replacement character U+FFFD is what lossy decode produces.
                assert_eq!(s.chars().count(), 2);
            }
            _ => panic!("expected Atom"),
        }
    });
}

#[test]
fn test_owned_float_conversion() {
    use fasteetf::owned::OwnedTerm;
    with_parse(b"\x83\x46\x40\x09\x21\xfb\x54\x44\x2d\x18", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Float(v) => assert!((v - core::f64::consts::PI).abs() < 1e-12),
            _ => panic!("expected Float"),
        }
    });
}

#[test]
fn test_owned_small_big_conversion() {
    use fasteetf::owned::OwnedTerm;
    with_parse(b"\x83\x6e\x02\x00\x01\x02", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::SmallBigInt { sign, digits } => {
                assert_eq!(sign, 0);
                assert_eq!(digits, vec![1, 2]);
            }
            _ => panic!("expected SmallBigInt, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_large_big_conversion() {
    use fasteetf::owned::OwnedTerm;
    with_parse(&[131, 111, 0, 0, 0, 2, 0, 0xAB, 0xCD], |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::LargeBigInt { sign, digits } => {
                assert_eq!(sign, 0);
                assert_eq!(digits, vec![0xAB, 0xCD]);
            }
            _ => panic!("expected LargeBigInt, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_binary_conversion() {
    use fasteetf::owned::OwnedTerm;
    let mut buf = vec![131, 109, 0, 0, 0, 3];
    buf.extend_from_slice(&[1, 2, 3]);
    with_parse(&buf, |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Binary(b) => assert_eq!(b, vec![1, 2, 3]),
            _ => panic!("expected Binary, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_bit_binary_conversion() {
    use fasteetf::owned::OwnedTerm;
    with_parse(&[131, 77, 0, 0, 0, 1, 3, 0b1010_0000], |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::BitBinary { bits, data } => {
                assert_eq!(bits, 3);
                assert_eq!(data, vec![0b1010_0000]);
            }
            _ => panic!("expected BitBinary, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_list_conversion() {
    use fasteetf::owned::OwnedTerm;
    // Proper list [1, 2].
    with_parse(&[131, 108, 0, 0, 0, 2, 97, 1, 97, 2, 106], |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::List(elems) => {
                assert_eq!(elems.len(), 2);
                assert!(matches!(elems[0], OwnedTerm::Int(1)));
                assert!(matches!(elems[1], OwnedTerm::Int(2)));
            }
            _ => panic!("expected List, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_improper_list_conversion() {
    use fasteetf::owned::OwnedTerm;
    // [1 | 2]
    with_parse(&[131, 108, 0, 0, 0, 1, 97, 1, 97, 2], |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::ImproperList { elements, tail } => {
                assert_eq!(elements.len(), 1);
                assert!(matches!(elements[0], OwnedTerm::Int(1)));
                assert!(matches!(*tail, OwnedTerm::Int(2)));
            }
            _ => panic!("expected ImproperList, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_map_conversion() {
    use fasteetf::owned::OwnedTerm;
    let mut buf = vec![131, 116, 0, 0, 0, 1];
    buf.extend_from_slice(&[97, 1]); // key: Int(1)
    buf.extend_from_slice(&[97, 2]); // val: Int(2)
    with_parse(&buf, |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Map(pairs) => {
                assert_eq!(pairs.len(), 1);
                assert!(matches!(pairs[0].0, OwnedTerm::Int(1)));
                assert!(matches!(pairs[0].1, OwnedTerm::Int(2)));
            }
            _ => panic!("expected Map, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_pid_conversion() {
    use fasteetf::owned::{OwnedTerm, PidOwned};
    with_parse(
        b"\x83\x67\x77\x04node\x00\x00\x00\x01\x00\x00\x00\x01\x01",
        |term| {
            let owned: OwnedTerm = term.into();
            match owned {
                OwnedTerm::Pid(PidOwned(tag, data)) => {
                    assert_eq!(tag, 103);
                    assert!(!data.is_empty());
                }
                _ => panic!("expected Pid, got {owned:?}"),
            }
        },
    );
}

#[test]
fn test_owned_port_conversion() {
    use fasteetf::owned::{OwnedTerm, PortOwned};
    with_parse(
        &[131, 102, 119, 4, b'n', b'o', b'd', b'e', 0, 0, 0, 1, 1],
        |term| {
            let owned: OwnedTerm = term.into();
            match owned {
                OwnedTerm::Port(PortOwned(tag, data)) => {
                    assert_eq!(tag, 102);
                    assert!(!data.is_empty());
                }
                _ => panic!("expected Port, got {owned:?}"),
            }
        },
    );
}

#[test]
fn test_owned_ref_conversion() {
    use fasteetf::owned::{OwnedTerm, ReferenceOwned};
    with_parse(
        &[
            131, 114, 0, 1, 119, 4, b'n', b'o', b'd', b'e', 1, 0, 0, 0, 7,
        ],
        |term| {
            let owned: OwnedTerm = term.into();
            match owned {
                OwnedTerm::Ref(ReferenceOwned(tag, data)) => {
                    assert_eq!(tag, 114);
                    assert!(!data.is_empty());
                }
                _ => panic!("expected Ref, got {owned:?}"),
            }
        },
    );
}

#[test]
fn test_owned_function_conversion() {
    use fasteetf::owned::{FunctionOwned, OwnedTerm};
    with_parse(b"\x83\x71\x77\x05lists\x77\x03map\x61\x02", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Function(FunctionOwned(tag, data)) => {
                assert_eq!(tag, 113);
                assert!(!data.is_empty());
            }
            _ => panic!("expected Function, got {owned:?}"),
        }
    });
}

#[test]
fn test_owned_record_conversion() {
    use fasteetf::owned::{OwnedTerm, RecordOwned};
    with_parse(
        b"\x83\x43\x00\x00\x00\x01\x01\x77\x03foo\x77\x03bar\x77\x01x\x61\x2a",
        |term| {
            let owned: OwnedTerm = term.into();
            match owned {
                OwnedTerm::Record(RecordOwned(data)) => {
                    assert!(!data.is_empty());
                }
                _ => panic!("expected Record, got {owned:?}"),
            }
        },
    );
}

#[test]
fn test_atom_utf8_lossy_string_conversion() {
    use fasteetf::owned::OwnedTerm;
    // UTF-8 atom 4 bytes: a "café"-like valid sequence.
    with_parse(b"\x83\x76\x00\x05hello", |term| {
        let owned: OwnedTerm = term.into();
        match owned {
            OwnedTerm::Atom(s) => assert_eq!(s, "hello"),
            _ => panic!("expected Atom"),
        }
    });
}
