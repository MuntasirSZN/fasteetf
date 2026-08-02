use super::*;

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

// ── New tags and conversions ────────────────────────────────────────────────

#[test]
fn test_new_port_ext() {
    // NEW_PORT_EXT (89): Node atom + 4-byte ID + 4-byte Creation.
    let mut buf = vec![131, 89];
    buf.extend_from_slice(b"\x77\x04node");
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    with_parse(&buf, |term| match term {
        Term::Port(data) => {
            assert_eq!(data.len(), 1 + 6 + 8); // 1 for tag + 6 for the atom + 8 for ID+Creation
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
            OwnedTerm::SmallBigInt { sign, digits } => {
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
                OwnedTerm::Pid(PidOwned(data)) => {
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
                OwnedTerm::Port(PortOwned(data)) => {
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
                OwnedTerm::Ref(ReferenceOwned(data)) => {
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
            OwnedTerm::Function(FunctionOwned(data)) => {
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

#[test]
fn test_term_size() {
    // Verify Term size is 24 bytes (optimized from 32 bytes)
    // This test verifies the size optimization
    let size = std::mem::size_of::<fasteetf::Term>();
    println!("Term size: {} bytes", size);
    assert_eq!(size, 24);
}
