use super::*;

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
        Term::ImproperList(elements) => {
            // New representation: last element is the tail
            assert_eq!(elements.len(), 2); // 1 element + 1 tail
            assert!(matches!(elements[0], Term::Int(1)));
            assert!(matches!(elements[1], Term::Int(2))); // tail
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
